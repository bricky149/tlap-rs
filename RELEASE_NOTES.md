# Version History

## v0.5.2 (2023-05-31)
* Update dependencies
* Fix build error caused by cpal changes
* Create a new subtitle file on every run instead of always appending
* Update copyright notices

## v0.5.1 (2022-11-04)
* Refactor streaming code to read only data written since the last read
* Split audio lines where we think there is silence, rather than every 64k samples

## v0.5.0 (2022-10-11)
* Add a test for subtitle output
* Change Options to Results as all data is needed during execution
* Reduce chance of panicking by covering for every Err() case
* Miscellaneous SemVer-breaking changes, despite no new features

## v0.4.0 (2022-10-07)
* Major codebase rewrite
	* Migrated from deprecated DeepSpeech dependencies to [coqui-stt](https://github.com/tazz4843/coqui-stt)
	* Migrated from PortAudio to cpal, allowing for cross-platform feature parity
	* Removed resampling functionality, cutting external crates almost in half
	* Separated speech-related and subtitle-related code into their own files
	* Reworked sub streaming as to not pin CPU usage to 100%
    * Threaded sub streaming to reduce transcription latency
* As Coqui does not offer CUDA binaries, CUDA support has been removed

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
* Added one-second lag to allow for DeepSpeech to determine words before they are saved
* Fixed subtitle pacing, previously it was averaging out words per subtitle which had sync issues
* Fixed an issue with some subtitles being written as 60 seconds rather than 00

## v0.1.0 (as wav2srt, 2021-12-15)
* Initial release
* Added ability to create subtitles based on a pre-recorded audio file, with optional words-per-minute argument
