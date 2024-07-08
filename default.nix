{ pkgs ? import <nixpkgs> { } }:

let manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;

  src = pkgs.lib.cleanSource ./.;

  buildInputs = [ pkgs.openssl pkgs.pkg-config ];
  meta = with pkgs.lib; {
    description =
      "A tool for managing encrypted environments for the age encryption tool";
    license = licenses.mit;
    maintainers = [ "jld.adriano@gmail.com" ];
  };
}
