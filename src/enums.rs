#[derive(Debug)]
pub enum TlapError {
	AudioSplitFailed,
	NoInputDevice,
	CreateRecordingFailed,
	NoInputStream,
	NoSpeechModel,
	InvalidSpeechModel,
	ReadFileFailed,
	ModelLockFailed,
	TranscriptionFailed,
	WriteSubtitlesFailed
}

pub enum TranscriptionType {
	Invalid,
	RealTime,
	PostRecord
}
