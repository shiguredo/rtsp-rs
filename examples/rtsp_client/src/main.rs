mod aac_depacketizer;
mod h264_depacketizer;
mod mp4_recorder;
mod sps;
#[cfg(feature = "video-display")]
mod video_player;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "video-display")]
use std::sync::mpsc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use shiguredo_http11::auth::DigestChallenge;
use shiguredo_http11::uri::Uri;
use shiguredo_rtsp::sdp::SdpAttribute;
use shiguredo_rtsp::{
    RtspClientConnection, RtspConnectionEvent, RtspConnectionState, RtspRequest, RtspStatusCode,
    Sdp,
};

use aac_depacketizer::AacDepacketizer;
use h264_depacketizer::H264Depacketizer;
use mp4_recorder::Mp4Recorder;
use shiguredo_rtsp::auth::{DigestCredentials, build_authorization};
#[cfg(feature = "video-display")]
use video_player::VideoDisplay;

/// チャネルとトラック情報のマッピング
struct TrackInfo {
    media_type: String,
    codec: String,
    rtp_channel: u8,
}

/// SDP fmtp パラメータから指定キーの値を取得する
fn get_fmtp_param<'a>(parameters: &'a str, key: &str) -> Option<&'a str> {
    for param in parameters.split(';') {
        let param = param.trim();
        if let Some((k, v)) = param.split_once('=')
            && k.trim().eq_ignore_ascii_case(key)
        {
            return Some(v.trim());
        }
    }
    None
}

/// SDP fmtp の sprop-parameter-sets から SPS/PPS を base64 デコードする
fn parse_sprop_parameter_sets(parameters: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let value = get_fmtp_param(parameters, "sprop-parameter-sets")?;
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() >= 2 {
        let sps = BASE64.decode(parts[0].trim()).ok()?;
        let pps = BASE64.decode(parts[1].trim()).ok()?;
        Some((sps, pps))
    } else {
        None
    }
}

/// AudioSpecificConfig からサンプルレートとチャネル数を抽出する
///
/// ISO/IEC 14496-3 Section 1.6.2.1
fn parse_audio_specific_config(config: &[u8]) -> Option<(u32, u16)> {
    if config.len() < 2 {
        return None;
    }

    // audioObjectType (5 bits) + samplingFrequencyIndex (4 bits) + channelConfiguration (4 bits)
    let audio_object_type = (config[0] >> 3) & 0x1F;
    let sampling_freq_index = ((config[0] & 0x07) << 1) | (config[1] >> 7);
    let mut channel_config;

    if audio_object_type == 31 {
        // extended audioObjectType (not typically used for AAC-LC)
        return None;
    }

    let sample_rate;
    if sampling_freq_index == 0x0F {
        // 24 bits samplingFrequency follows
        if config.len() < 5 {
            return None;
        }
        sample_rate = ((config[1] as u32 & 0x7F) << 17)
            | ((config[2] as u32) << 9)
            | ((config[3] as u32) << 1)
            | (config[4] as u32 >> 7);
        channel_config = (config[4] >> 3) & 0x0F;
    } else {
        static SAMPLE_RATES: [u32; 13] = [
            96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
        ];
        if sampling_freq_index as usize >= SAMPLE_RATES.len() {
            return None;
        }
        sample_rate = SAMPLE_RATES[sampling_freq_index as usize];
        channel_config = (config[1] >> 3) & 0x0F;
    }

    if channel_config == 0 {
        // channel_config 0 は program_config_element で定義されるが、
        // 一般的には stereo を仮定する
        channel_config = 2;
    }

    Some((sample_rate, channel_config as u16))
}

fn flush_send_buf(stream: &mut TcpStream, conn: &mut RtspClientConnection) -> std::io::Result<()> {
    let send_data = conn.send_buf();
    if !send_data.is_empty() {
        stream.write_all(send_data)?;
        stream.flush()?;
        let len = send_data.len();
        conn.advance_send_buf(len);
    }
    Ok(())
}

/// 受信データを読み取り、接続に渡す
///
/// WouldBlock/TimedOut の場合は false を返し、データ受信時は true を返す。
fn recv_and_process(
    stream: &mut TcpStream,
    conn: &mut RtspClientConnection,
    buf: &mut [u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    match stream.read(buf) {
        Ok(0) => Err("connection closed by server".into()),
        Ok(n) => {
            conn.feed_recv_buf(&buf[..n])?;
            Ok(true)
        }
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            Ok(false)
        }
        Err(e) => Err(e.into()),
    }
}

fn wait_for_response(
    stream: &mut TcpStream,
    conn: &mut RtspClientConnection,
    buf: &mut [u8],
) -> Result<shiguredo_rtsp::RtspResponse, Box<dyn std::error::Error>> {
    loop {
        while let Some(event) = conn.next_event() {
            match event {
                RtspConnectionEvent::ResponseReceived(response) => {
                    return Ok(response);
                }
                RtspConnectionEvent::StateChanged(state) => {
                    println!("[STATE] {:?}", state);
                }
                _ => {}
            }
        }
        recv_and_process(stream, conn, buf)?;
    }
}

/// リクエストを送信し、401 応答時に Digest 認証付きでリトライする
fn send_with_auth(
    stream: &mut TcpStream,
    conn: &mut RtspClientConnection,
    buf: &mut [u8],
    request: RtspRequest,
    credentials: &Option<DigestCredentials>,
) -> Result<shiguredo_rtsp::RtspResponse, Box<dyn std::error::Error>> {
    conn.send_request(request.clone())?;
    flush_send_buf(stream, conn)?;
    let response = wait_for_response(stream, conn, buf)?;

    if response.status_code == RtspStatusCode::Unauthorized as u16
        && let Some(creds) = credentials
        && let Some(www_auth) = response.get_header("WWW-Authenticate")
        && let Ok(challenge) = DigestChallenge::parse(www_auth)
    {
        println!("[AUTH] 401 Unauthorized, retrying with Digest auth");
        let auth_value =
            build_authorization(creds, &challenge, request.method.as_str(), &request.uri);
        let auth_request = request.header("Authorization", &auth_value);
        conn.send_request(auth_request)?;
        flush_send_buf(stream, conn)?;
        return wait_for_response(stream, conn, buf);
    }

    Ok(response)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rtsp-url> [output.mp4]", args[0]);
        eprintln!("Example: {} rtsp://localhost:8554/test output.mp4", args[0]);
        std::process::exit(1);
    }

    let url = &args[1];
    let output_path = args.get(2).map(|s| s.as_str());

    println!("Connecting to: {}", url);
    if let Some(path) = output_path {
        println!("MP4 output: {}", path);
    }

    // Ctrl+C ハンドリング
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    setup_signal_handler(&running_clone);

    // URL パース
    let uri = Uri::parse(url)?;
    let host = uri.host().ok_or("URL にホストが含まれていない")?;
    let port = uri.port().unwrap_or(554);

    // URL から認証情報を抽出する (userinfo@host 形式)
    let credentials = extract_credentials(&uri);
    if credentials.is_some() {
        println!("Digest auth credentials found in URL");
    }

    let mut stream = TcpStream::connect((host, port))?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let mut conn = RtspClientConnection::new();
    let mut buf = [0u8; 65536];

    // OPTIONS
    println!("\n--- OPTIONS ---");
    let request = RtspRequest::new(shiguredo_rtsp::RtspMethod::Options, url);
    let response = send_with_auth(&mut stream, &mut conn, &mut buf, request, &credentials)?;
    println!("{}", response);
    if let Some(public) = response.get_header("Public") {
        println!("Public: {}", public);
    }

    // DESCRIBE
    println!("\n--- DESCRIBE ---");
    let request = RtspRequest::new(shiguredo_rtsp::RtspMethod::Describe, url)
        .header("Accept", "application/sdp");
    let response = send_with_auth(&mut stream, &mut conn, &mut buf, request, &credentials)?;
    println!("{}", response);

    if !response.is_success() {
        return Err(format!(
            "DESCRIBE failed: {} {}",
            response.status_code, response.reason_phrase
        )
        .into());
    }

    let sdp_text = String::from_utf8_lossy(&response.body);
    println!("SDP:\n{}", sdp_text);
    let sdp = Sdp::parse(&sdp_text)?;

    // SDP からトラック情報を解析する
    let mut tracks: Vec<TrackInfo> = Vec::new();
    let mut h264_depacketizer = H264Depacketizer::new();
    let mut aac_depacketizer: Option<AacDepacketizer> = None;

    // MP4 レコーダーの初期化
    let mut recorder: Option<Mp4Recorder> = match output_path {
        Some(path) => {
            let rec = Mp4Recorder::new(Path::new(path))?;
            Some(rec)
        }
        None => None,
    };

    // 映像表示の初期化用に SPS/PPS と解像度を保持する
    #[cfg(feature = "video-display")]
    let mut video_sps: Option<Vec<u8>> = None;
    #[cfg(feature = "video-display")]
    let mut video_pps: Option<Vec<u8>> = None;

    // 各メディアトラックを SETUP (interleaved)
    let mut channel = 0u8;
    for media in &sdp.media {
        let track_uri = media
            .attributes
            .iter()
            .find_map(|a| {
                if let SdpAttribute::Control(ctrl) = a {
                    Some(ctrl.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let setup_uri = if track_uri.starts_with("rtsp://") {
            track_uri
        } else {
            let base = response.get_header("Content-Base").unwrap_or(url.as_str());
            let base = base.trim_end_matches('/');
            format!("{}/{}", base, track_uri)
        };

        // rtpmap からコーデック情報を取得する
        let mut codec = String::new();
        let mut _clock_rate = 0u32;
        for attr in &media.attributes {
            if let SdpAttribute::Rtpmap {
                encoding,
                clock_rate,
                ..
            } = attr
            {
                codec = encoding.clone();
                _clock_rate = *clock_rate;
                break;
            }
        }

        // fmtp からコーデック固有パラメータを取得する
        for attr in &media.attributes {
            if let SdpAttribute::Fmtp { parameters, .. } = attr {
                if codec == "H264" {
                    // H.264: sprop-parameter-sets から SPS/PPS を取得
                    if let Some((sps, pps)) = parse_sprop_parameter_sets(parameters) {
                        println!(
                            "H.264 SPS/PPS extracted from SDP (SPS: {} bytes, PPS: {} bytes)",
                            sps.len(),
                            pps.len()
                        );
                        #[cfg(feature = "video-display")]
                        {
                            video_sps = Some(sps.clone());
                            video_pps = Some(pps.clone());
                        }
                        if let Some(ref mut rec) = recorder {
                            rec.set_sps_pps(sps, pps);
                        }
                    }
                } else if codec.eq_ignore_ascii_case("MPEG4-GENERIC") {
                    // AAC: config, sizeLength, indexLength を取得
                    let size_length: u8 = get_fmtp_param(parameters, "sizeLength")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(13);
                    let index_length: u8 = get_fmtp_param(parameters, "indexLength")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(3);

                    aac_depacketizer = Some(AacDepacketizer::new(size_length, index_length));

                    if let Some(config_hex) = get_fmtp_param(parameters, "config")
                        && let Ok(config) = hex_decode(config_hex)
                        && let Some((sample_rate, channels)) = parse_audio_specific_config(&config)
                    {
                        println!(
                            "AAC config: sample_rate={}, channels={}",
                            sample_rate, channels
                        );
                        if let Some(ref mut rec) = recorder {
                            rec.set_aac_config(config, sample_rate, channels);
                        }
                    }
                }
                break;
            }
        }

        println!(
            "\n--- SETUP {} ({}) [{}] ---",
            media.media_type, setup_uri, codec
        );
        let transport = format!(
            "RTP/AVP/TCP;unicast;interleaved={}-{}",
            channel,
            channel + 1
        );
        let request = RtspRequest::new(shiguredo_rtsp::RtspMethod::Setup, &setup_uri)
            .header("Transport", &transport);
        let resp = send_with_auth(&mut stream, &mut conn, &mut buf, request, &credentials)?;
        println!("{}", resp);

        if !resp.is_success() {
            return Err(
                format!("SETUP failed: {} {}", resp.status_code, resp.reason_phrase).into(),
            );
        }

        tracks.push(TrackInfo {
            media_type: media.media_type.clone(),
            codec: codec.clone(),
            rtp_channel: channel,
        });

        channel += 2;
    }

    // PLAY
    println!("\n--- PLAY ---");
    let request =
        RtspRequest::new(shiguredo_rtsp::RtspMethod::Play, url).header("Range", "npt=0.000-");
    let response = send_with_auth(&mut stream, &mut conn, &mut buf, request, &credentials)?;
    println!("{}", response);

    if !response.is_success() {
        return Err(format!(
            "PLAY failed: {} {}",
            response.status_code, response.reason_phrase
        )
        .into());
    }

    // 映像表示の初期化
    #[cfg(feature = "video-display")]
    let mut video_display: Option<VideoDisplay> = {
        let openh264_path = Path::new("libopenh264-2.6.0-mac-arm64.dylib");
        if openh264_path.exists() {
            let (width, height) = video_sps
                .as_ref()
                .and_then(|sps| sps::parse_sps(sps))
                .map(|info| (info.width as i32, info.height as i32))
                .unwrap_or((1920, 1080));

            match VideoDisplay::new(openh264_path, width, height, "RTSP Client") {
                Ok(mut display) => {
                    if let (Some(sps), Some(pps)) = (&video_sps, &video_pps) {
                        display.set_sps_pps(sps, pps);
                    }
                    Some(display)
                }
                Err(e) => {
                    eprintln!("Video display init failed: {}", e);
                    None
                }
            }
        } else {
            println!("OpenH264 library not found, video display disabled");
            None
        }
    };

    // RTSP 受信を別スレッドで実行し、アクセスユニットを mpsc チャネルで送る
    #[cfg(feature = "video-display")]
    let (au_tx, au_rx) = mpsc::channel::<h264_depacketizer::AccessUnit>();
    let recv_running = running.clone();
    let recv_url = url.to_string();

    let recv_thread = std::thread::spawn(move || {
        stream.set_read_timeout(Some(std::time::Duration::from_millis(10)))?;

        let mut rtp_count = 0u64;
        let mut rtcp_count = 0u64;

        while recv_running.load(Ordering::Relaxed) {
            match recv_and_process(&mut stream, &mut conn, &mut buf) {
                Ok(_) => {}
                Err(e) => {
                    println!("Receive error: {}", e);
                    break;
                }
            }

            while let Some(event) = conn.next_event() {
                match event {
                    RtspConnectionEvent::RtpReceived { channel, packet } => {
                        rtp_count += 1;

                        let track = tracks.iter().find(|t| t.rtp_channel == channel);

                        match track {
                            Some(t) if t.codec == "H264" => {
                                if rtp_count % 100 == 1 {
                                    println!(
                                        "RTP [H264] ch={} seq={} ts={} (total: {})",
                                        channel,
                                        packet.header.sequence_number,
                                        packet.header.timestamp,
                                        rtp_count
                                    );
                                }

                                #[allow(clippy::collapsible_if)]
                                if let Some(au) = h264_depacketizer.push(
                                    &packet.payload,
                                    packet.header.timestamp,
                                    packet.header.marker,
                                ) {
                                    if let Some(ref mut rec) = recorder {
                                        rec.update_sps_pps_if_available(
                                            h264_depacketizer.sps.as_ref(),
                                            h264_depacketizer.pps.as_ref(),
                                        );
                                        if let Err(e) = rec.write_access_unit(&au) {
                                            eprintln!("MP4 video write error: {}", e);
                                        }
                                    }
                                    // メインスレッドに送信 (受信側が閉じていたら終了)
                                    #[cfg(feature = "video-display")]
                                    if au_tx.send(au).is_err() {
                                        recv_running.store(false, Ordering::Relaxed);
                                    }
                                }
                            }
                            Some(t) if t.codec.eq_ignore_ascii_case("MPEG4-GENERIC") => {
                                if rtp_count % 100 == 1 {
                                    println!(
                                        "RTP [AAC] ch={} seq={} ts={} (total: {})",
                                        channel,
                                        packet.header.sequence_number,
                                        packet.header.timestamp,
                                        rtp_count
                                    );
                                }

                                if let Some(ref mut depkt) = aac_depacketizer {
                                    let frames =
                                        depkt.push(&packet.payload, packet.header.timestamp);
                                    if let Some(ref mut rec) = recorder {
                                        for frame in &frames {
                                            if let Err(e) = rec.write_aac_frame(frame) {
                                                eprintln!("MP4 audio write error: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                            Some(t) => {
                                if rtp_count % 100 == 1 {
                                    println!(
                                        "RTP [{}] ch={} seq={} ts={} pt={} (total: {})",
                                        t.media_type,
                                        channel,
                                        packet.header.sequence_number,
                                        packet.header.timestamp,
                                        packet.header.payload_type,
                                        rtp_count
                                    );
                                }
                            }
                            None => {}
                        }
                    }
                    RtspConnectionEvent::RtcpReceived { channel, packets } => {
                        rtcp_count += 1;
                        println!(
                            "RTCP ch={} packets={} (total: {})",
                            channel,
                            packets.len(),
                            rtcp_count
                        );
                    }
                    RtspConnectionEvent::StateChanged(state) => {
                        println!("[STATE] {:?}", state);
                        if state == RtspConnectionState::Disconnected {
                            println!("Disconnected");
                            recv_running.store(false, Ordering::Relaxed);
                        }
                    }
                    RtspConnectionEvent::ResponseReceived(resp) => {
                        println!("Response: {}", resp);
                    }
                    _ => {}
                }
            }
        }

        // MP4 ファイナライズ
        if let Some(rec) = recorder.take() {
            println!("Finalizing MP4...");
            match rec.finalize() {
                Ok(()) => println!("MP4 recording finalized"),
                Err(e) => eprintln!("MP4 finalize error: {}", e),
            }
        }

        // TEARDOWN
        println!("\n--- TEARDOWN ---");
        let _ = conn.send_teardown(&recv_url);
        let _ = flush_send_buf(&mut stream, &mut conn);

        println!("RTP packets received: {}", rtp_count);
        println!("RTCP packets received: {}", rtcp_count);

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // メインスレッド: 映像デコード・表示 + イベントループ
    println!("\n--- Receiving media (Ctrl+C to stop) ---");

    #[cfg(feature = "video-display")]
    while running.load(Ordering::Relaxed) {
        // チャネルからアクセスユニットを非ブロッキングで受信する
        while let Ok(au) = au_rx.try_recv() {
            if let Some(ref mut display) = video_display
                && let Err(e) = display.display_access_unit(&au)
            {
                eprintln!("Video display error: {}", e);
            }
        }

        // SDL イベント処理と画面更新
        if let Some(ref display) = video_display {
            match display.poll_events() {
                Ok(false) => {
                    println!("Window closed");
                    running.store(false, Ordering::Relaxed);
                    break;
                }
                Err(e) => {
                    eprintln!("Video display error: {}", e);
                    running.store(false, Ordering::Relaxed);
                    break;
                }
                _ => {}
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    #[cfg(not(feature = "video-display"))]
    while running.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // 映像表示を終了する
    #[cfg(feature = "video-display")]
    drop(video_display);

    // 受信スレッドの終了を待つ
    running.store(false, Ordering::Relaxed);
    if let Err(e) = recv_thread.join().expect("recv thread panicked") {
        eprintln!("Receive thread error: {}", e);
    }

    Ok(())
}

/// シグナルハンドラを設定する
///
/// SIGINT (Ctrl+C) と SIGTERM を受信したらフラグを false にして受信ループを終了させる。
fn setup_signal_handler(running: &Arc<AtomicBool>) {
    // シグナルハンドラ内ではシグナル安全な操作のみ許可されるため、
    // グローバル AtomicBool にフラグを立てて、監視スレッドで running を更新する
    unsafe {
        libc_signal(2, signal_handler); // SIGINT
        libc_signal(15, signal_handler); // SIGTERM
    }
    let r = running.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if SIGNAL_RECEIVED.load(Ordering::Relaxed) {
                r.store(false, Ordering::Relaxed);
                break;
            }
        }
    });
}

static SIGNAL_RECEIVED: AtomicBool = AtomicBool::new(false);

extern "C" fn signal_handler(_sig: std::ffi::c_int) {
    SIGNAL_RECEIVED.store(true, Ordering::Relaxed);
}

unsafe fn libc_signal(sig: std::ffi::c_int, handler: extern "C" fn(std::ffi::c_int)) {
    // POSIX signal(3)
    unsafe {
        unsafe extern "C" {
            fn signal(sig: std::ffi::c_int, handler: extern "C" fn(std::ffi::c_int)) -> *const ();
        }
        signal(sig, handler);
    }
}

/// URL の authority 部分から認証情報を抽出する
///
/// `rtsp://user:pass@host/path` 形式の URL から username と password を取得する。
fn extract_credentials(uri: &Uri) -> Option<DigestCredentials> {
    let authority = uri.authority()?;
    let (userinfo, _host) = authority.rsplit_once('@')?;
    let (username, password) = userinfo.split_once(':')?;
    Some(DigestCredentials {
        username: username.to_string(),
        password: password.to_string(),
    })
}

/// 16 進数文字列をバイト列にデコードする
fn hex_decode(hex: &str) -> Result<Vec<u8>, ()> {
    let hex = hex.trim();
    if !hex.len().is_multiple_of(2) {
        return Err(());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ()))
        .collect()
}
