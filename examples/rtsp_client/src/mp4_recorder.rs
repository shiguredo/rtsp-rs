//! shiguredo_mp4 を使った MP4 録画モジュール
//!
//! H.264 アクセスユニットと AAC フレームを受け取り、MP4 ファイルに書き込む。

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::num::NonZeroU32;
use std::path::Path;
use std::time::SystemTime;

use shiguredo_mp4::FixedPointNumber;
use shiguredo_mp4::TrackKind;
use shiguredo_mp4::Uint;
use shiguredo_mp4::boxes::{
    AudioSampleEntryFields, Avc1Box, AvccBox, EsdsBox, Mp4aBox, SampleEntry,
    VisualSampleEntryFields,
};
use shiguredo_mp4::descriptors::{
    DecoderConfigDescriptor, DecoderSpecificInfo, EsDescriptor, SlConfigDescriptor,
};
use shiguredo_mp4::mux::{Mp4FileMuxer, Mp4FileMuxerOptions, Sample};

use crate::aac_depacketizer::AacFrame;
use crate::h264_depacketizer::AccessUnit;
use crate::sps::SpsInfo;

/// H.264 の RTP clock rate
const VIDEO_TIMESCALE: u32 = 90000;
/// AAC の 1 フレームあたりのサンプル数 (AAC-LC)
const AAC_SAMPLES_PER_FRAME: u32 = 1024;

pub struct Mp4Recorder {
    file: File,
    muxer: Mp4FileMuxer,
    /// 現在のファイル書き込み位置
    file_position: u64,
    // H.264 関連
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
    sps_info: Option<SpsInfo>,
    video_first_sample: bool,
    pending_au: Option<PendingAccessUnit>,
    // AAC 関連
    aac_config: Option<Vec<u8>>,
    aac_sample_rate: u32,
    aac_channels: u16,
    audio_first_sample: bool,
}

struct PendingAccessUnit {
    data: Vec<u8>,
    timestamp: u32,
    keyframe: bool,
}

impl Mp4Recorder {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let creation_timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let options = Mp4FileMuxerOptions {
            creation_timestamp,
            ..Default::default()
        };
        let muxer = Mp4FileMuxer::with_options(options)?;
        let initial_bytes = muxer.initial_boxes_bytes();

        let mut file = File::create(path)?;
        file.write_all(initial_bytes)?;
        let file_position = initial_bytes.len() as u64;

        Ok(Self {
            file,
            muxer,
            file_position,
            sps: None,
            pps: None,
            sps_info: None,
            video_first_sample: true,
            pending_au: None,
            aac_config: None,
            aac_sample_rate: 0,
            aac_channels: 0,
            audio_first_sample: true,
        })
    }

    /// SDP から取得した SPS/PPS を設定する
    pub fn set_sps_pps(&mut self, sps: Vec<u8>, pps: Vec<u8>) {
        self.sps_info = crate::sps::parse_sps(&sps);
        self.sps = Some(sps);
        self.pps = Some(pps);
    }

    /// AAC の設定情報を設定する
    pub fn set_aac_config(&mut self, config: Vec<u8>, sample_rate: u32, channels: u16) {
        self.aac_config = Some(config);
        self.aac_sample_rate = sample_rate;
        self.aac_channels = channels;
    }

    /// インバンドの SPS/PPS を更新する
    pub fn update_sps_pps_if_available(&mut self, sps: Option<&Vec<u8>>, pps: Option<&Vec<u8>>) {
        if let Some(sps) = sps
            && self.sps.as_ref() != Some(sps)
        {
            self.sps_info = crate::sps::parse_sps(sps);
            self.sps = Some(sps.clone());
        }
        if let Some(pps) = pps
            && self.pps.as_ref() != Some(pps)
        {
            self.pps = Some(pps.clone());
        }
    }

    /// H.264 アクセスユニットを書き込む
    pub fn write_access_unit(&mut self, au: &AccessUnit) -> Result<(), Box<dyn std::error::Error>> {
        if self.sps.is_none() || self.pps.is_none() || self.sps_info.is_none() {
            return Ok(());
        }

        if let Some(pending) = self.pending_au.take() {
            let duration = au.timestamp.wrapping_sub(pending.timestamp);
            self.write_video_sample(&pending.data, pending.keyframe, duration)?;
        }

        self.pending_au = Some(PendingAccessUnit {
            data: au.data.clone(),
            timestamp: au.timestamp,
            keyframe: au.keyframe,
        });

        Ok(())
    }

    /// AAC フレームを書き込む
    pub fn write_aac_frame(&mut self, frame: &AacFrame) -> Result<(), Box<dyn std::error::Error>> {
        if self.aac_config.is_none() {
            return Ok(());
        }

        self.file.write_all(&frame.data)?;

        let sample_entry = if self.audio_first_sample {
            self.audio_first_sample = false;
            Some(self.build_audio_sample_entry())
        } else {
            None
        };

        let sample = Sample {
            track_kind: TrackKind::Audio,
            sample_entry,
            keyframe: true,
            timescale: NonZeroU32::new(self.aac_sample_rate).expect("サンプルレートは非ゼロ"),
            duration: AAC_SAMPLES_PER_FRAME,
            data_offset: self.file_position,
            data_size: frame.data.len(),
        };
        self.muxer.append_sample(&sample)?;

        self.file_position += frame.data.len() as u64;

        Ok(())
    }

    fn write_video_sample(
        &mut self,
        data: &[u8],
        keyframe: bool,
        duration: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.file.write_all(data)?;

        let sample_entry = if self.video_first_sample {
            self.video_first_sample = false;
            Some(self.build_video_sample_entry())
        } else {
            None
        };

        let sample = Sample {
            track_kind: TrackKind::Video,
            sample_entry,
            keyframe,
            timescale: NonZeroU32::new(VIDEO_TIMESCALE).expect("非ゼロ"),
            duration,
            data_offset: self.file_position,
            data_size: data.len(),
        };
        self.muxer.append_sample(&sample)?;

        self.file_position += data.len() as u64;

        Ok(())
    }

    fn build_video_sample_entry(&self) -> SampleEntry {
        let sps_info = self.sps_info.as_ref().expect("SPS 情報が必要");
        let sps = self.sps.as_ref().expect("SPS が必要");
        let pps = self.pps.as_ref().expect("PPS が必要");

        let is_high_profile = !matches!(sps_info.profile_idc, 66 | 77 | 88);

        let avcc_box = AvccBox {
            avc_profile_indication: sps_info.profile_idc,
            profile_compatibility: sps_info.profile_compatibility,
            avc_level_indication: sps_info.level_idc,
            length_size_minus_one: Uint::new(3),
            sps_list: vec![sps.clone()],
            pps_list: vec![pps.clone()],
            chroma_format: if is_high_profile {
                Some(Uint::new(sps_info.chroma_format_idc))
            } else {
                None
            },
            bit_depth_luma_minus8: if is_high_profile {
                Some(Uint::new(sps_info.bit_depth_luma_minus8))
            } else {
                None
            },
            bit_depth_chroma_minus8: if is_high_profile {
                Some(Uint::new(sps_info.bit_depth_chroma_minus8))
            } else {
                None
            },
            sps_ext_list: vec![],
        };

        SampleEntry::Avc1(Avc1Box {
            visual: VisualSampleEntryFields {
                data_reference_index: VisualSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
                width: sps_info.width,
                height: sps_info.height,
                horizresolution: VisualSampleEntryFields::DEFAULT_HORIZRESOLUTION,
                vertresolution: VisualSampleEntryFields::DEFAULT_VERTRESOLUTION,
                frame_count: VisualSampleEntryFields::DEFAULT_FRAME_COUNT,
                compressorname: VisualSampleEntryFields::NULL_COMPRESSORNAME,
                depth: VisualSampleEntryFields::DEFAULT_DEPTH,
            },
            avcc_box,
            unknown_boxes: vec![],
        })
    }

    fn build_audio_sample_entry(&self) -> SampleEntry {
        let aac_config = self.aac_config.as_ref().expect("AAC config が必要");

        SampleEntry::Mp4a(Mp4aBox {
            audio: AudioSampleEntryFields {
                data_reference_index: AudioSampleEntryFields::DEFAULT_DATA_REFERENCE_INDEX,
                channelcount: self.aac_channels,
                samplesize: AudioSampleEntryFields::DEFAULT_SAMPLESIZE,
                samplerate: FixedPointNumber::new(self.aac_sample_rate as u16, 0),
            },
            esds_box: EsdsBox {
                es: EsDescriptor {
                    es_id: 2,
                    stream_priority: EsDescriptor::LOWEST_STREAM_PRIORITY,
                    depends_on_es_id: None,
                    url_string: None,
                    ocr_es_id: None,
                    dec_config_descr: DecoderConfigDescriptor {
                        object_type_indication:
                            DecoderConfigDescriptor::OBJECT_TYPE_INDICATION_AUDIO_ISO_IEC_14496_3,
                        stream_type: DecoderConfigDescriptor::STREAM_TYPE_AUDIO,
                        up_stream: DecoderConfigDescriptor::UP_STREAM_FALSE,
                        buffer_size_db: Uint::new(0),
                        max_bitrate: 0,
                        avg_bitrate: 0,
                        dec_specific_info: Some(DecoderSpecificInfo {
                            payload: aac_config.clone(),
                        }),
                    },
                    sl_config_descr: SlConfigDescriptor,
                },
            },
            unknown_boxes: vec![],
        })
    }

    /// 録画を完了して MP4 ファイルをファイナライズする
    pub fn finalize(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // バッファリング中の最後のビデオフレームを書き出す
        if let Some(pending) = self.pending_au.take() {
            // 最後のフレームは duration を推定する (1/30 秒 = 3000 @ 90kHz)
            let duration = 3000;
            self.write_video_sample(&pending.data, pending.keyframe, duration)?;
        }

        let finalized = self.muxer.finalize()?;
        for (offset, bytes) in finalized.offset_and_bytes_pairs() {
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.write_all(bytes)?;
        }

        self.file.flush()?;
        Ok(())
    }
}
