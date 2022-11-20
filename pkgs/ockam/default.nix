# Replace "stdenv" with the namespace or name of your language's builder
{ self,
  stdenv,
  lib,
  fetchurl,
}:

stdenv.mkDerivation rec {
  pname = "ockam";
  version = "0.77.0";
  MACOSX_DEPLOYMENT_TARGET = true;

  sourceRoot = ".";

  src = fetchurl {
    url = "https://github.com/build-trust/ockam/releases/download/ockam_v${version}/ockam.x86_64-apple-darwin";
    sha256 = "sha256-Kjt3rXFu+AI8cSVxrerlaLwIq3K5dmgO9pT0a+Lqjjs=";
    executable = true;
  };

  phases = ["installPhase"];
  installPhase = ''
    mkdir -p $out/bin
    cp $src $out/bin/ockam
    chmod +x $out/bin/ockam
  '';

  meta = with lib; {
    homepage = "https://github.com/build-trust/ockam/";
    description = "ockam binary";
    platforms = platforms.darwin;
    architectures = [ "x86" ];
  };

}
