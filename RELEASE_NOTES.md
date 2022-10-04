# Version History

## v0.3.2 (2021-12-18)
* Now working with CUDA!
* Audio buffer no longer split into chunks as it resulted in missed words
* Code tidy-up as per clippy recommendations
* Micro-optimisations to avoid unnecessary memory allocations

## v0.3.1 (2021-12-17)
* Real-time subtitles will now have its associated audio recorded
* Reset the previous timestamp variable if the timespan to write a subtitle has been missed to avoid being chronically behind

## v0.3.0 (as tlap, 2021-12-17)
* Post-record subtitling code has been rewritten to transliterate chunks of audio at a time rather than the whole thing at once
* Micro-optimisation to reduce audio loop latency
* Added usage notes when run without arguments
* Removed one-second lag as it was better to drop the last word in a transliteration, allowing a more correct word in the next one
* Fixed a panic that would occur if nothing was interpreted within the first five seconds

## v0.2.0 (2021-12-16)
* Added ability to create subtitles based on live input via PortAudio
* Added one-second lag to allow for DeepSpeech to determine words before they're saved
* Fixed subtitle pacing, previously it was averaging out words per subtitle which had sync issues
* Fixed an issue with some subtitles being written as 60 seconds rather than 00

## v0.1.0 (as wav2srt, 2021-12-15)
* Initial release
* Added ability to create subtitles based on a pre-recorded audio file, with optional words-per-minute argument