{ pkgs ? import <nixpkgs> { } }:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "oxidots";
  version = "0.1";

  cargoLock.lockFile = ./Cargo.lock;

  src = pkgs.lib.cleanSource ./.;
}
