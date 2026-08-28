{ stdenv, cargo-reaper }:
stdenv.mkDerivation {
  name = "test-cargo-reaper-new-ext";
  buildInputs = [
    cargo-reaper
  ];
  doCheck = true;
  phases = [
    "buildPhase"
    "checkPhase"
    "installPhase"
  ];
  buildPhase = ''
    cargo-reaper new --template ext reaper_test
  '';
  checkPhase = ''
    if [ ! -d "reaper_test" ]; then
      exit 1
    fi
  '';
  installPhase = ''
    mkdir -p $out
    mv reaper_test $out/
  '';
}
