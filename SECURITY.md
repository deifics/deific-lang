# Security Policy

## Scope

Deific is a transpiler that generates C++ from `.df` source files. Security concerns include:

- **Compiler bugs** that silently produce incorrect output (e.g. wrong integer overflow behaviour)
- **Arbitrary file write** vulnerabilities in the compiler itself
- **Code injection** via maliciously crafted `.df` source passed to the compiler

The *generated* C++ and compiled binaries are the user's own code and are out of scope.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Email **dineshsinnath@gmail.com** with:
- A description of the vulnerability
- Steps to reproduce (minimal `.df` file if applicable)
- Potential impact

You can expect an acknowledgement within 72 hours and a fix or mitigation plan within 14 days.
