{ inputs, lib, stdenv, rustPlatform, fetchFromGitHub, pkg-config, makeWrapper, curl, openssl, darwin }:

rustPlatform.buildRustPackage rec {
  pname = "cargo-tarpaulin";
  version = "develop";

  src = fetchFromGitHub {
    owner = "xd009642";
    repo = "tarpaulin";
    rev = version;
    sha256 = "sha256-MvQQiEIXPkFsG9omMPk5Se3OujYuOl/ePQFiGwoPlNw";
  };

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];
  buildInputs = [ openssl ]
    ++ lib.optionals stdenv.isDarwin [
      curl
      darwin.apple_sdk.frameworks.Security
    ];

   postInstall = lib.optionalString stdenv.isDarwin ''
    wrapProgram $out/bin/cargo-tarpaulin --set PATH "${inputs.fenix.packages.latest.cargo}/bin:${inputs.fenix.packages.latest.rustc}/bin:$PATH"
  '';

  cargoSha256 = "sha256-AfvY8vC9uh3HwJc3y6GsGqsJmjkVcCJ2gHKdMSTCw84=";
  #checkFlags = [ "--test-threads" "1" ];
  doCheck = false;

  meta = with lib; {
    description = "A code coverage tool for Rust projects";
    homepage = "https://github.com/xd009642/tarpaulin";
    license = with licenses; [ mit /* or */ asl20 ];
    maintainers = with maintainers; [ hugoreeves ];
  };
}
