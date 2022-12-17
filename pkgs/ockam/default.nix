{ self,
  lib,
  rust_1_65,
  hostPlatform,
  dbus,
  openssl,
  pkg-config,
  libiconv,
  darwin,
  inputs,
  callPackage,
  evcxr,
  cargo-tarpaulin,
  clippy,
  rustfmt,
}:
let
  inherit (rust_1_65.packages.stable) rustPlatform;
  cargo-tarpaulin-develop =
    if hostPlatform.isDarwin then
      callPackage ./cargo-tarpaulin-darwin.nix { inherit inputs; }
    else cargo-tarpaulin;
in rustPlatform.buildRustPackage {
  # this is necessary for vs code / rust-analyzer to find the rust-src library
  RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";

  pname = "ockam";
  version = "0.77.0"; # XXX: should get that from Cargo.lock
  src = self;
  doCheck = false; # disable tests requiring network access
  cargoLock = {
    lockFile = self + "/Cargo.lock";
    outputHashes = {
      #   "dependency-0.0.0" = lib.fakeSha256;
    };
  };
  buildInputs =
    [
      openssl.dev
      dbus.dev
    ] ++ lib.optional hostPlatform.isDarwin [
      libiconv
      darwin.apple_sdk.frameworks.Security
      darwin.apple_sdk.frameworks.DiskArbitration
      darwin.apple_sdk.frameworks.Foundation
    ] ++ lib.optional hostPlatform.isLinux [ ];
  nativeBuildInputs = [
    pkg-config # for openssl
    evcxr
    clippy
    rustfmt
    cargo-tarpaulin-develop
  ];
}
