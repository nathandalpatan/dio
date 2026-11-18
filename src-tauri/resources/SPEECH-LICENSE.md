# Speech attribution

## Kokoro-82M voice model

DIO does not include the voice model. When you choose **Install voice** in
Settings, DIO downloads `onnx/model_quantized.onnx` and six voice files from
[onnx-community/Kokoro-82M-v1.0-ONNX](https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX)
at commit `1939ad2a8e416c0acfeecc08a694d14ef25f2231`. It then verifies each
file against a SHA-256 checksum pinned in DIO's source. The model is an ONNX
export of [hexgrad/Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M) and is
licensed under the
[Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).

## Misaki G2P

DIO converts text to Kokoro phonemes with
[misaki-rs](https://github.com/MicheleYin/misaki-rs), a Rust port of
[hexgrad/misaki](https://github.com/hexgrad/misaki) (Apache License 2.0) that
embeds Misaki's English lexicons. DIO builds misaki-rs without its optional
eSpeak NG fallback. Words outside the lexicon are read from CMUdict or spelled
out letter by letter.

```
MIT License

Copyright (c) 2026 Michele Yin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## ONNX Runtime

`onnxruntime.dll` is Microsoft ONNX Runtime 1.31.1 for Windows x64 (CPU),
extracted from the official `Microsoft.ML.OnnxRuntime` NuGet package by
`tools/fetch_onnxruntime.py`. Its MIT license is in `ONNXRUNTIME-LICENSE.txt`,
and its third-party notices are in `ONNXRUNTIME-THIRD-PARTY-NOTICES.txt`. DIO
turns off ONNX Runtime telemetry.
