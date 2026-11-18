# Pronunciation data attribution

`pronunciations.sqlite3` holds two tables, built with
`tools/build_pronunciation_dictionary.py`.

## Wiktionary IPA

The `wiktionary` table contains 150,454 phonemic IPA transcriptions for 94,177
English headwords. They were extracted from the
[English JSONL export](https://kaikki.org/dictionary/English/index.html) of
English Wiktionary published by Tatu Ylonen's
[Wiktextract/Kaikki](https://kaikki.org/dictionary/) project (file dated
2026-10-30, gzip SHA-256
`4d03b6cba283744756c57ff661c2c6cbfe34fcf8ff513a966576cf7d97e91a99`).

DIO kept only slash-delimited (phonemic) transcriptions. A transcription is
labelled US when its Wiktionary tags include `US` or `General-American` and no
UK tag, and UK when they include `UK` or `Received-Pronunciation` and no US tag.
Untagged transcriptions are kept without an accent label. Transcriptions tagged
only with other regions, or as rare, obsolete, dated, dialectal or nonstandard,
were dropped, as were fragments. At most three are kept per accent.

Copyright Wiktionary contributors. This modified subset is available under
[Creative Commons Attribution-ShareAlike 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

## CMUdict

The `cmudict` table contains 135,166 ARPABET entries from the
[CMU Pronouncing Dictionary](https://github.com/cmusphinx/cmudict) at commit
`74790861f652b15e4ac49015a90074ad62a27690` (`cmudict.dict` SHA-256
`81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22`). DIO
converts entries to American IPA at runtime and labels them as converted from
CMUdict.

```
Copyright (C) 1993-2015 Carnegie Mellon University. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.
   The contents of this file are deemed to be source code.

2. Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in
   the documentation and/or other materials provided with the
   distribution.

This work was supported in part by funding from the Defense Advanced
Research Projects Agency, the Office of Naval Research and the National
Science Foundation of the United States of America, and by member
companies of the Carnegie Mellon Sphinx Speech Consortium. We acknowledge
the contributions of many volunteers to the expansion and improvement of
this dictionary.

THIS SOFTWARE IS PROVIDED BY CARNEGIE MELLON UNIVERSITY ``AS IS'' AND
ANY EXPRESSED OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL CARNEGIE MELLON UNIVERSITY
NOR ITS EMPLOYEES BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```
