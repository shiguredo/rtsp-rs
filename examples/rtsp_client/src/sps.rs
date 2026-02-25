//! H.264 SPS (Sequence Parameter Set) パーサー
//!
//! ITU-T H.264 7.3.2.1.1 準拠。
//! SPS NAL ユニットから映像の幅・高さおよびプロファイル情報を抽出する。

/// SPS から抽出した情報
pub struct SpsInfo {
    pub profile_idc: u8,
    pub profile_compatibility: u8,
    pub level_idc: u8,
    pub chroma_format_idc: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub width: u16,
    pub height: u16,
}

/// ビット単位の読み取りリーダー
struct BitReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    fn read_bits(&mut self, n: u8) -> Option<u32> {
        let mut val: u32 = 0;
        for _ in 0..n {
            if self.byte_offset >= self.data.len() {
                return None;
            }
            let bit = (self.data[self.byte_offset] >> (7 - self.bit_offset)) & 1;
            val = (val << 1) | bit as u32;
            self.bit_offset += 1;
            if self.bit_offset == 8 {
                self.bit_offset = 0;
                self.byte_offset += 1;
            }
        }
        Some(val)
    }

    fn read_bit(&mut self) -> Option<u32> {
        self.read_bits(1)
    }

    /// Exp-Golomb 符号 unsigned (9.1)
    fn read_ue(&mut self) -> Option<u32> {
        let mut leading_zeros: u32 = 0;
        while let 0 = self.read_bit()? {
            leading_zeros += 1;
            if leading_zeros > 31 {
                return None;
            }
        }
        if leading_zeros == 0 {
            return Some(0);
        }
        let suffix = self.read_bits(leading_zeros as u8)?;
        Some((1 << leading_zeros) - 1 + suffix)
    }

    /// Exp-Golomb 符号 signed (9.1.1)
    fn read_se(&mut self) -> Option<i32> {
        let code = self.read_ue()?;
        let abs_val = code.div_ceil(2);
        if code % 2 == 0 {
            Some(-(abs_val as i32))
        } else {
            Some(abs_val as i32)
        }
    }
}

/// RBSP からエミュレーション防止バイト (0x00 0x00 0x03) を除去する
fn remove_emulation_prevention(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x03 {
            out.push(0x00);
            out.push(0x00);
            i += 3;
        } else {
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

/// SPS NAL ユニットをパースする
///
/// `data` は NAL ヘッダーバイト (0x67 等) を含む SPS NAL ユニット全体。
pub fn parse_sps(data: &[u8]) -> Option<SpsInfo> {
    if data.is_empty() {
        return None;
    }

    let nal_type = data[0] & 0x1F;
    if nal_type != 7 {
        return None;
    }

    let rbsp = remove_emulation_prevention(&data[1..]);
    let mut r = BitReader::new(&rbsp);

    let profile_idc = r.read_bits(8)? as u8;
    let profile_compatibility = r.read_bits(8)? as u8;
    let level_idc = r.read_bits(8)? as u8;
    let _seq_parameter_set_id = r.read_ue()?;

    let mut chroma_format_idc: u8 = 1;
    let mut bit_depth_luma_minus8: u8 = 0;
    let mut bit_depth_chroma_minus8: u8 = 0;

    let high_profiles = [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135];
    if high_profiles.contains(&profile_idc) {
        chroma_format_idc = r.read_ue()? as u8;
        if chroma_format_idc == 3 {
            let _separate_colour_plane_flag = r.read_bits(1)?;
        }
        bit_depth_luma_minus8 = r.read_ue()? as u8;
        bit_depth_chroma_minus8 = r.read_ue()? as u8;
        let _qpprime_y_zero_transform_bypass_flag = r.read_bits(1)?;
        let seq_scaling_matrix_present_flag = r.read_bits(1)?;
        if seq_scaling_matrix_present_flag == 1 {
            let count = if chroma_format_idc != 3 { 8 } else { 12 };
            for i in 0..count {
                let present = r.read_bits(1)?;
                if present == 1 {
                    let size = if i < 6 { 16 } else { 64 };
                    skip_scaling_list(&mut r, size)?;
                }
            }
        }
    }

    let _log2_max_frame_num_minus4 = r.read_ue()?;
    let pic_order_cnt_type = r.read_ue()?;
    match pic_order_cnt_type {
        0 => {
            let _log2_max_pic_order_cnt_lsb_minus4 = r.read_ue()?;
        }
        1 => {
            let _delta_pic_order_always_zero_flag = r.read_bits(1)?;
            let _offset_for_non_ref_pic = r.read_se()?;
            let _offset_for_top_to_bottom_field = r.read_se()?;
            let num_ref_frames_in_pic_order_cnt_cycle = r.read_ue()?;
            for _ in 0..num_ref_frames_in_pic_order_cnt_cycle {
                let _offset = r.read_se()?;
            }
        }
        _ => {}
    }

    let _max_num_ref_frames = r.read_ue()?;
    let _gaps_in_frame_num_value_allowed_flag = r.read_bits(1)?;
    let pic_width_in_mbs_minus1 = r.read_ue()?;
    let pic_height_in_map_units_minus1 = r.read_ue()?;
    let frame_mbs_only_flag = r.read_bits(1)?;

    if frame_mbs_only_flag == 0 {
        let _mb_adaptive_frame_field_flag = r.read_bits(1)?;
    }

    let _direct_8x8_inference_flag = r.read_bits(1)?;
    let frame_cropping_flag = r.read_bits(1)?;

    let (crop_left, crop_right, crop_top, crop_bottom) = if frame_cropping_flag == 1 {
        (r.read_ue()?, r.read_ue()?, r.read_ue()?, r.read_ue()?)
    } else {
        (0, 0, 0, 0)
    };

    // SubWidthC / SubHeightC (Table 6-1)
    let (sub_width_c, sub_height_c) = match chroma_format_idc {
        0 => (1u32, 1u32),
        1 => (2, 2),
        2 => (2, 1),
        3 => (1, 1),
        _ => (2, 2),
    };

    let crop_unit_x = sub_width_c;
    let crop_unit_y = sub_height_c * (2 - frame_mbs_only_flag);

    let width =
        ((pic_width_in_mbs_minus1 + 1) * 16).saturating_sub(crop_unit_x * (crop_left + crop_right));
    let height = ((pic_height_in_map_units_minus1 + 1) * 16 * (2 - frame_mbs_only_flag))
        .saturating_sub(crop_unit_y * (crop_top + crop_bottom));

    Some(SpsInfo {
        profile_idc,
        profile_compatibility,
        level_idc,
        chroma_format_idc,
        bit_depth_luma_minus8,
        bit_depth_chroma_minus8,
        width: width as u16,
        height: height as u16,
    })
}

fn skip_scaling_list(r: &mut BitReader<'_>, size: usize) -> Option<()> {
    let mut last_scale: i32 = 8;
    let mut next_scale: i32 = 8;
    for _ in 0..size {
        if next_scale != 0 {
            let delta_scale = r.read_se()?;
            next_scale = (last_scale + delta_scale + 256) % 256;
        }
        last_scale = if next_scale == 0 {
            last_scale
        } else {
            next_scale
        };
    }
    Some(())
}
