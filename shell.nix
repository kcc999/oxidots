{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell rec {
  buildInputs = [
    pkgs.rustc
    pkgs.cargo
    pkgs.rust-analyzer
    pkgs.pkg-config
    pkgs.openssl.dev
  ];

  shellHook = ''
    export PATH=${pkgs.rust-analyzer}/bin:$PATH
    export RUST_SRC_PATH=${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}
  '';
}

