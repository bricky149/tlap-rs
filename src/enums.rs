#[derive(Debug)]
pub enum TlapError {
	AudioSplitFailed,
	NoInputDevice,
	CreateRecordingFailed,
	NoInputStream,
	ReadFileFailed,
	ModelLockFailed,
	TranscriptionFailed,
	WriteRecordingFailed,
	WriteSubtitlesFailed
}

pub enum TranscriptionType {
	Invalid,
	RealTime,
	PostRecord
}
