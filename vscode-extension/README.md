# TT Graph CDFA VS Code Extension

Prototype VS Code integration for the Rust TT Graph CDFA analyzer.

## Setup

Build the Rust analyzer from the repository root:

```powershell
cargo build
```

Install extension dependencies:

```powershell
cd vscode-extension
npm install
npm run compile
```

Open this folder in VS Code, press `F5`, and run the extension in the Extension
Development Host.

## Usage

Commands:

- `TT Graph CDFA: Analyze Current File`
- `TT Graph CDFA: Analyze Workspace Example`
- `TT Graph CDFA: Show Graph`
- `TT Graph CDFA: Clear Diagnostics`

The extension reads the analyzer binary from:

```json
"ttGraphCdfa.analyzerPath": ""
```

If this setting is empty, it tries the repository-local debug binary under
`target/debug/`.

## Current Scope

- C++ paper examples via `diagnostics-cpp`
- implicit `std::thread` example via `diagnostics-cpp-implicit`
- Problems diagnostics
- SVG TT Graph webview
- click graph nodes to jump to source locations when available

This is an IDE/SDE prototype. It is not a full C++ language server and does not
yet handle arbitrary multi-file CMake projects.
