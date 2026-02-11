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

Type "oag --help"
Enter
Sleep 3s

Type "oag init"
Enter
Sleep 2s

Type "cat .urmzd.oag.yaml"
Enter
Sleep 3s

Type "oag validate -i crates/oag-core/tests/fixtures/petstore-3.2.yaml"
Enter
Sleep 2s

Type "oag generate -i crates/oag-core/tests/fixtures/petstore-3.2.yaml -o out -t typescript"
Enter
Sleep 2s

Type "ls out/"
Enter
Sleep 2s

Type "head -40 out/types.ts"
Enter
Sleep 4s

Type "rm -rf out .urmzd.oag.yaml"
Enter
Sleep 1s
