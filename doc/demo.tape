# oag demo — OpenAPI 3.x code generator
# Requires: vhs (https://github.com/charmbracelet/vhs)

Output doc/demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 1000
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"

# Ensure `oag` is on PATH (e.g. cargo install oag-cli, or ./target/release)
Type "export PATH=./target/release:$PATH"
Enter
Sleep 1s

# Show CLI help
Type "oag --help"
Enter
Sleep 3s

# Validate a spec
Type "oag validate -i crates/oag-core/tests/fixtures/petstore-3.2.yaml"
Enter
Sleep 2s

# Inspect the IR
Type "oag inspect -i crates/oag-core/tests/fixtures/petstore-3.2.yaml"
Enter
Sleep 4s

# Generate TypeScript client
Type "oag generate -i crates/oag-core/tests/fixtures/petstore-3.2.yaml -o out -t typescript"
Enter
Sleep 2s

# Show what was generated
Type "ls out/"
Enter
Sleep 2s

# Show the generated types
Type "cat out/types.ts"
Enter
Sleep 4s

# Show the generated client (the interesting methods)
Type "cat out/client.ts"
Enter
Sleep 5s

# Clean up
Type "rm -rf out"
Enter
Sleep 1s
