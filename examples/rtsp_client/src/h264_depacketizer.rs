//! RFC 6184 H.264 RTP デパケタイザー
//!
//! RTP パケットから H.264 アクセスユニットを再構成する。
//! 対応 NAL タイプ:
//!   - 1-23: Single NAL Unit
//!   - 24: STAP-A (集約パケット)
//!   - 28: FU-A (フラグメンテーションユニット)

/// 再構成されたアクセスユニット
pub struct AccessUnit {
    /// AVCC 形式のデータ (4byte 長プレフィックス + NAL ユニット)
    pub data: Vec<u8>,
    /// RTP タイムスタンプ
    pub timestamp: u32,
    /// キーフレーム (IDR) かどうか
    pub keyframe: bool,
}

pub struct H264Depacketizer {
    /// 現在蓄積中の AVCC データ
    current_data: Vec<u8>,
    /// 現在のアクセスユニットの RTP タイムスタンプ
    current_ts: u32,
    /// 現在のアクセスユニットにキーフレームが含まれるか
    current_keyframe: bool,
    /// FU-A フラグメントバッファ
    fua_buf: Vec<u8>,
    /// FU-A 処理中かどうか
    fua_active: bool,
    /// インバンドで検出された SPS
    pub sps: Option<Vec<u8>>,
    /// インバンドで検出された PPS
    pub pps: Option<Vec<u8>>,
}

impl H264Depacketizer {
    pub fn new() -> Self {
        Self {
            current_data: Vec::new(),
            current_ts: 0,
            current_keyframe: false,
            fua_buf: Vec::new(),
            fua_active: false,
            sps: None,
            pps: None,
        }
    }

    /// RTP パケットを処理する
    ///
    /// `marker` が true のとき、蓄積済みデータをアクセスユニットとして返す。
    pub fn push(&mut self, payload: &[u8], timestamp: u32, marker: bool) -> Option<AccessUnit> {
        if payload.is_empty() {
            return None;
        }

        // タイムスタンプが変わった場合、現在の蓄積データをリセット
        if !self.current_data.is_empty() && self.current_ts != timestamp {
            self.current_data.clear();
            self.current_keyframe = false;
        }
        self.current_ts = timestamp;

        let nal_type = payload[0] & 0x1F;

        match nal_type {
            // Single NAL Unit (1-23)
            1..=23 => {
                self.process_single_nal(payload);
            }
            // STAP-A
            24 => {
                self.process_stap_a(payload);
            }
            // FU-A
            28 => {
                self.process_fua(payload);
            }
            _ => {
                // 未対応 NAL タイプは無視
            }
        }

        if marker && !self.current_data.is_empty() {
            let au = AccessUnit {
                data: std::mem::take(&mut self.current_data),
                timestamp,
                keyframe: self.current_keyframe,
            };
            self.current_keyframe = false;
            Some(au)
        } else {
            None
        }
    }

    fn process_single_nal(&mut self, nal: &[u8]) {
        let nal_type = nal[0] & 0x1F;
        self.check_parameter_set(nal_type, nal);
        self.check_keyframe(nal_type);
        self.append_avcc(nal);
    }

    fn process_stap_a(&mut self, payload: &[u8]) {
        // STAP-A ヘッダーの 1 バイト目をスキップ
        let mut offset = 1;
        while offset + 2 <= payload.len() {
            let size = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
            offset += 2;
            if offset + size > payload.len() {
                break;
            }
            let nal = &payload[offset..offset + size];
            if !nal.is_empty() {
                let nal_type = nal[0] & 0x1F;
                self.check_parameter_set(nal_type, nal);
                self.check_keyframe(nal_type);
                self.append_avcc(nal);
            }
            offset += size;
        }
    }

    fn process_fua(&mut self, payload: &[u8]) {
        if payload.len() < 2 {
            return;
        }

        let fu_indicator = payload[0];
        let fu_header = payload[1];
        let start = (fu_header & 0x80) != 0;
        let end = (fu_header & 0x40) != 0;
        let nal_type = fu_header & 0x1F;

        if start {
            // FU-A 開始: NAL ヘッダーを再構成
            let nal_header = (fu_indicator & 0xE0) | nal_type;
            self.fua_buf.clear();
            self.fua_buf.push(nal_header);
            self.fua_buf.extend_from_slice(&payload[2..]);
            self.fua_active = true;
        } else if self.fua_active {
            // FU-A 継続
            self.fua_buf.extend_from_slice(&payload[2..]);
        }

        if end && self.fua_active {
            self.fua_active = false;
            let nal_type_inner = self.fua_buf[0] & 0x1F;
            self.check_parameter_set(nal_type_inner, &self.fua_buf.clone());
            self.check_keyframe(nal_type_inner);
            let buf = std::mem::take(&mut self.fua_buf);
            self.append_avcc(&buf);
        }
    }

    /// AVCC 形式 (4byte 長プレフィックス + NAL) を蓄積バッファに追加する
    fn append_avcc(&mut self, nal: &[u8]) {
        let len = nal.len() as u32;
        self.current_data.extend_from_slice(&len.to_be_bytes());
        self.current_data.extend_from_slice(nal);
    }

    fn check_parameter_set(&mut self, nal_type: u8, nal: &[u8]) {
        match nal_type {
            7 => self.sps = Some(nal.to_vec()),
            8 => self.pps = Some(nal.to_vec()),
            _ => {}
        }
    }

    fn check_keyframe(&mut self, nal_type: u8) {
        if nal_type == 5 {
            self.current_keyframe = true;
        }
    }
}
