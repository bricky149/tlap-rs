## tlap

### What is tlap?
tlap (transliterate language for an accessibility purpose) is a program that can take live or pre-recorded audio and transliterate that into a subtitle file following the SRT format.

### Why is it called 'tlap'?
'tlap' is an in-joke from the Fediverse based on a user of a similar name, 'tlapka'.

### Why did you make it?
My university records lectures but do not include subtitles. This makes it difficult for those hard of hearing to follow along and impedes the process of summarising what the lecture covers.

### What audio formats are supported?
Only 16-bit single-channel (mono) Wave is supported. This program will output to 16-bit mono Wave format if real-time subtitling has been specified.

### Where can I get the source code for Coqui?
You may find the source code [here](https://github.com/coqui-ai/STT/).

### Are the bundled Coqui binaries modified?
No, the binaries used were obtained from the project's [releases page](https://github.com/coqui-ai/STT/releases/).

## Prerequisites

### Packages
The following list is for Fedora (38). Ensure packages offering similar functionality for your distro are installed.

`rust cargo glibc-devel alsa-lib-devel`

### libstt
You will need to download the libs from [Coqui's releases page](https://github.com/coqui-ai/STT/releases/) and modify the LIBRARY_PATH in 'perform_stt.sh'. On Linux I would suggest /usr/local/lib, on Windows I have bundled binaries as a suggested place to run them from.

### Model (mandatory) and scorer (optional)
Coqui (formerly DeepSpeech) works by using a pre-trained model to determine what has been said in a given audio sample. To improve speed and accuracy it may use a scorer, a vocabulary bank of sorts, to pick a word that fits.
