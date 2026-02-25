//! OpenH264 デコーダー + raw_player による映像表示モジュール
//!
//! H.264 アクセスユニット (AVCC 形式) をデコードしてウィンドウに表示する。

use std::path::Path;
use std::time::Instant;

use raw_player::VideoPlayer;
use shiguredo_openh264::{Decoder, Openh264Library};

use crate::h264_depacketizer::AccessUnit;

/// 映像デコード・表示を管理する
pub struct VideoDisplay {
    decoder: Decoder,
    player: VideoPlayer,
    /// SPS (Annex.B 形式、スタートコード付き)
    sps: Option<Vec<u8>>,
    /// PPS (Annex.B 形式、スタートコード付き)
    pps: Option<Vec<u8>>,
    /// 最初のフレームかどうか
    first_frame: bool,
    /// 表示フレーム数
    frame_count: u64,
    /// 最初のフレームデコード時刻 (PTS の基準)
    start_time: Option<Instant>,
}

impl VideoDisplay {
    /// OpenH264 ライブラリをロードして映像表示を初期化する
    pub fn new(
        openh264_path: &Path,
        width: i32,
        height: i32,
        title: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let lib = Openh264Library::load(openh264_path)?;
        println!(
            "OpenH264 loaded: {} ({})",
            openh264_path.display(),
            lib.runtime_version()
        );

        let decoder = Decoder::new(lib)?;

        raw_player::init()?;
        let player = VideoPlayer::new(width, height, title)?;
        player.play()?;

        Ok(Self {
            decoder,
            player,
            sps: None,
            pps: None,
            first_frame: true,
            frame_count: 0,
            start_time: None,
        })
    }

    /// SDP から取得した SPS/PPS を設定する
    pub fn set_sps_pps(&mut self, sps: &[u8], pps: &[u8]) {
        // Annex.B スタートコード + NAL データ
        let mut sps_annexb = vec![0, 0, 0, 1];
        sps_annexb.extend_from_slice(sps);
        self.sps = Some(sps_annexb);

        let mut pps_annexb = vec![0, 0, 0, 1];
        pps_annexb.extend_from_slice(pps);
        self.pps = Some(pps_annexb);
    }

    /// H.264 アクセスユニットをデコードして表示する
    ///
    /// アクセスユニットは AVCC 形式 (4byte 長プレフィックス + NAL) で格納されている。
    /// OpenH264 は Annex.B 形式 (スタートコード付き) を要求するため変換する。
    pub fn display_access_unit(
        &mut self,
        au: &AccessUnit,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut annexb = Vec::new();

        // キーフレームの場合は SPS/PPS を先頭に付加する
        if au.keyframe || self.first_frame {
            if let Some(ref sps) = self.sps {
                annexb.extend_from_slice(sps);
            }
            if let Some(ref pps) = self.pps {
                annexb.extend_from_slice(pps);
            }
            self.first_frame = false;
        }

        // AVCC → Annex.B 変換
        annexb.extend_from_slice(&avcc_to_annexb(&au.data));

        // デコード
        match self.decoder.decode(&annexb) {
            Ok(Some(frame)) => {
                let width = frame.width() as i32;
                let height = frame.height() as i32;

                // ストライドを考慮して連続バッファを作成する
                let y = compact_plane(
                    frame.y_plane(),
                    frame.y_stride(),
                    width as usize,
                    height as usize,
                );
                let u = compact_plane(
                    frame.u_plane(),
                    frame.u_stride(),
                    width as usize / 2,
                    height as usize / 2,
                );
                let v = compact_plane(
                    frame.v_plane(),
                    frame.v_stride(),
                    width as usize / 2,
                    height as usize / 2,
                );

                let pts_us = self
                    .start_time
                    .get_or_insert_with(Instant::now)
                    .elapsed()
                    .as_micros() as i64;
                self.player
                    .enqueue_video_i420(&y, &u, &v, width, height, pts_us)?;
                self.frame_count += 1;

                if self.frame_count.is_multiple_of(30) {
                    println!(
                        "[VIDEO] decoded frame {} ({}x{})",
                        self.frame_count, width, height
                    );
                }
            }
            Ok(None) => {
                if self.frame_count < 10 || self.frame_count.is_multiple_of(100) {
                    println!(
                        "[VIDEO] decode returned None (keyframe={}, annexb_len={})",
                        au.keyframe,
                        annexb.len()
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "[VIDEO] decode error: {} (keyframe={}, annexb_len={})",
                    e,
                    au.keyframe,
                    annexb.len()
                );
            }
        }

        Ok(())
    }

    /// イベントを処理する
    ///
    /// ウィンドウが閉じられた場合は false を返す。
    pub fn poll_events(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.player.poll_events()?)
    }
}

impl Drop for VideoDisplay {
    fn drop(&mut self) {
        raw_player::quit();
    }
}

/// AVCC 形式 (4byte 長プレフィックス) を Annex.B 形式 (スタートコード) に変換する
fn avcc_to_annexb(data: &[u8]) -> Vec<u8> {
    let mut annexb = Vec::with_capacity(data.len());
    let mut pos = 0;

    while pos + 4 <= data.len() {
        let len =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if pos + len > data.len() {
            break;
        }

        annexb.extend_from_slice(&[0, 0, 0, 1]);
        annexb.extend_from_slice(&data[pos..pos + len]);
        pos += len;
    }

    annexb
}

/// ストライド付きプレーンデータを連続バッファにコピーする
fn compact_plane(plane: &[u8], stride: usize, width: usize, height: usize) -> Vec<u8> {
    if stride == width {
        return plane[..width * height].to_vec();
    }

    let mut compacted = Vec::with_capacity(width * height);
    for row in 0..height {
        let start = row * stride;
        if start + width <= plane.len() {
            compacted.extend_from_slice(&plane[start..start + width]);
        }
    }
    compacted
}
