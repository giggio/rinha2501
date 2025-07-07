{ pkgs ? import <nixpkgs> { }, ... }:

pkgs.mkShell {
  name = "rinha2501";
  nativeBuildInputs = with pkgs; [
    (fenix.stable.withComponents [
      "cargo"
      "rust-src"
      "clippy"
      "rustfmt"
      "rust-analyzer"
      "rustc"
    ])
    rust-bindgen
    mask
    bacon
    k6
  ];
  shellHook = ''
    echo "Let's Rinha!"
  '';
}
