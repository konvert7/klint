"""Drive an interactive CLI under a pseudo-terminal.

Usage: pty-driver.py <binary> <workdir> <comma-separated-args> [step ...]
Steps are `expect:<text>` (wait for text on screen) or `send:<key>`.
Prints everything the program wrote; exits 1 if an expectation times out.
"""

from __future__ import annotations

import fcntl
import os
import pty
import re
import select
import signal
import struct
import sys
import termios
import time

KEYS = {
    "enter": b"\r",
    "esc": b"\x1b",
    "space": b" ",
    "down": b"\x1b[B",
    "up": b"\x1b[A",
    "left": b"\x1b[D",
    "right": b"\x1b[C",
}
ANSI = re.compile(r"\x1b\[[0-9;?]*[a-zA-Z]")
EXPECT_TIMEOUT_SECONDS = 20.0


def main() -> int:
    binary, workdir, argstr, *steps = sys.argv[1:]
    cli_args = [arg for arg in argstr.split(",") if arg]

    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(workdir)
        os.execv(binary, [binary, *cli_args])

    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 100, 0, 0))
    session = Session(fd)

    try:
        for step in steps:
            action, _, value = step.partition(":")
            if action == "expect":
                if not session.wait_for(value):
                    sys.stdout.write(session.text())
                    sys.stderr.write(f"pty-driver: timed out waiting for {value!r}\n")
                    return 1
            elif action == "send":
                os.write(fd, KEYS[value])
            else:
                sys.stderr.write(f"pty-driver: unknown step {step!r}\n")
                return 1
        session.drain_until_exit()
    finally:
        terminate(pid, fd)

    sys.stdout.write(session.text())
    return 0


class Session:
    def __init__(self, fd: int) -> None:
        self.fd = fd
        self.buffer = bytearray()

    def text(self) -> str:
        return ANSI.sub("", self.buffer.decode(errors="replace"))

    def read_once(self, timeout: float) -> bool:
        ready, _, _ = select.select([self.fd], [], [], timeout)
        if not ready:
            return False
        try:
            chunk = os.read(self.fd, 4096)
        except OSError:
            return False
        if not chunk:
            return False
        self.buffer.extend(chunk)
        return True

    def wait_for(self, needle: str) -> bool:
        deadline = time.time() + EXPECT_TIMEOUT_SECONDS
        while needle not in self.text():
            if time.time() > deadline:
                return False
            self.read_once(0.2)
        return True

    def drain_until_exit(self) -> None:
        deadline = time.time() + EXPECT_TIMEOUT_SECONDS
        while time.time() < deadline:
            if not self.read_once(0.3):
                return


def terminate(pid: int, fd: int) -> None:
    try:
        os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    os.waitpid(pid, 0)
    os.close(fd)


if __name__ == "__main__":
    raise SystemExit(main())
