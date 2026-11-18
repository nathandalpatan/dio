"""Extract the pinned Windows x64 CPU ONNX Runtime from Microsoft's official NuGet package."""

import hashlib
import sys
import urllib.request
import zipfile
from io import BytesIO
from pathlib import Path


VERSION = "1.31.1"
PACKAGE_URL = (
    "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/"
    f"{VERSION}/microsoft.ml.onnxruntime.{VERSION}.nupkg"
)
PACKAGE_SHA256 = "6871ccbc97f3d5bdb4095a1dcec33aa78d56f6ed8de5980ef5fa1d9f2e7c679a"
FILES = {
    "runtimes/win-x64/native/onnxruntime.dll": "onnxruntime.dll",
    "LICENSE": "ONNXRUNTIME-LICENSE.txt",
    "ThirdPartyNotices.txt": "ONNXRUNTIME-THIRD-PARTY-NOTICES.txt",
}


def main() -> None:
    if len(sys.argv) not in (2, 3):
        raise SystemExit("usage: fetch_onnxruntime.py <resources-dir> [downloaded.nupkg]")
    target = Path(sys.argv[1])
    if len(sys.argv) == 3:
        package = Path(sys.argv[2]).read_bytes()
    else:
        with urllib.request.urlopen(PACKAGE_URL) as response:
            package = response.read()
    if hashlib.sha256(package).hexdigest() != PACKAGE_SHA256:
        raise ValueError("ONNX Runtime package checksum does not match the pinned release")
    with zipfile.ZipFile(BytesIO(package)) as archive:
        for source, name in FILES.items():
            (target / name).write_bytes(archive.read(source))
    print(f"Extracted ONNX Runtime {VERSION} into {target}")


if __name__ == "__main__":
    main()
