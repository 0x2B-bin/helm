{
  rustPlatform
}:
rustPlatform.buildRustPackage {
  pname = "helm";
  version = "0.1.0";
  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };
}
