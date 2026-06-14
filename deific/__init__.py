"""
deific — Python helper for calling Deific programs as subprocesses.

Quick start:

    import deific

    result = deific.run("solver.df", input="5\n1 2 3 4 5\n")
    print(result.stdout)

    # Stream a large file directly (no memory copy):
    deific.pipe("solver.df", input_file="big_input.txt")
"""

import subprocess
import shutil
import os
from dataclasses import dataclass
from typing import Optional


def _deific_exe() -> str:
    """Return the path to the deific binary."""
    exe = shutil.which("deific")
    if exe:
        return exe
    # Common local build locations
    for candidate in [
        os.path.join(os.path.dirname(__file__), "..", "target", "release", "deific.exe"),
        os.path.join(os.path.dirname(__file__), "..", "target", "release", "deific"),
    ]:
        if os.path.isfile(candidate):
            return os.path.abspath(candidate)
    raise FileNotFoundError(
        "deific binary not found. Build it with: cargo +stable-x86_64-pc-windows-gnu build --release"
    )


@dataclass
class Result:
    stdout: str
    stderr: str
    returncode: int

    def check(self) -> "Result":
        """Raise RuntimeError if the program exited non-zero."""
        if self.returncode != 0:
            raise RuntimeError(
                f"deific program exited {self.returncode}.\nstderr:\n{self.stderr}"
            )
        return self


def run(
    source: str,
    input: Optional[str] = None,
    input_file: Optional[str] = None,
    timeout: Optional[float] = None,
) -> Result:
    """
    Compile and run a .df source file, capture stdout/stderr.

    Args:
        source:     Path to the .df file.
        input:      String to feed as stdin (mutually exclusive with input_file).
        input_file: Path to a file to feed as stdin (more efficient for large inputs).
        timeout:    Seconds before the subprocess is killed (None = no limit).

    Returns:
        Result(stdout, stderr, returncode)
    """
    if input is not None and input_file is not None:
        raise ValueError("provide either input= or input_file=, not both")

    exe = _deific_exe()

    if input_file is not None:
        # Use deific pipe so the compiler handles the redirect natively
        proc = subprocess.run(
            [exe, "pipe", source, "--input", input_file],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    else:
        proc = subprocess.run(
            [exe, "run", source],
            input=input,
            capture_output=True,
            text=True,
            timeout=timeout,
        )

    return Result(stdout=proc.stdout, stderr=proc.stderr, returncode=proc.returncode)


def pipe(
    source: str,
    input_file: Optional[str] = None,
    timeout: Optional[float] = None,
) -> int:
    """
    Compile and run a .df file, inheriting the current process's stdin/stdout/stderr.
    Useful when you want the program's output to stream directly to the terminal.

    Returns the exit code.
    """
    exe = _deific_exe()
    cmd = [exe, "pipe", source]
    if input_file is not None:
        cmd += ["--input", input_file]

    proc = subprocess.run(cmd, timeout=timeout)
    return proc.returncode
