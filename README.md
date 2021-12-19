## tlap

### What is tlap?
tlap (transliterate language for an accessibility purpose) is a program that can take live or pre-recorded audio and transliterate that into a subtitle file following the SRT format.

### Why is it called 'tlap'?
'tlap' is an in-joke from the Fediverse based on a user of a similar name, 'tlapka'.

### Why did you make it?
My university records lectures but do not include subtitles. This makes it difficult for those hard of hearing to follow along and impedes the process of summarising what the lecture covers.

### What audio formats are supported?
Either 16-bit single-channel (mono) Wave or Ogg Vorbis formats are accepted inputs. This program will output to 16-bit mono Wave format if real-time subtitling has been specified. Any inputs that need it will be resampled to 16kHz.

### Where can I get the source code for DeepSpeech?
You may find the source code [here](https://github.com/mozilla/DeepSpeech).

### Are the bundled DeepSpeech binaries modified?
No, the binaries used were obtained from the project's [releases page](https://github.com/mozilla/DeepSpeech/releases).

## Prerequisites

### Rust (latest stable)
You need to install Rust either via rustup.rs or from your Linux distribution's software repository.

### gcc, make, pkgconf
Required for compiling PortAudio and its associated crate.

### Model and scorer
DeepSpeech (now Coqui, pending new bindings) works by using a pre-trained model to determine what has been said in a given audio sample. To improve its accuracy it may use a scorer, a vocabulary bank of sorts, to pick a word that fits. While a scorer is usually optional this program mandates having one as not having it reduces the program's value.

### CUDA runtime and libraries (optional)
DeepSpeech can run either on the CPU (software) or GPU (CUDA). If you want the latter you will need to install version 10.1 of Nvidia's GPU Computing Toolkit as the DeepSpeech binaries distributed with this software were linked to that particular version.

## Tips

### Recording computer output as input (via pavucontrol)
Go to the Configuration tab and change your default sound card's Profile to Analog Stereo Output. This will make PortAudio listen to the output device as input.
