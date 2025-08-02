{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell rec {
  buildInputs = [
    pkgs.rustc
    pkgs.cargo
    pkgs.rust-analyzer
  ];

  shellHook = ''
    export PATH=${pkgs.rust-analyzer}/bin:$PATH
    export RUST_SRC_PATH=${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}
  '';
}

