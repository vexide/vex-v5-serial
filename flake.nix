{
  description = "A Rust implementation of the V5 Serial Protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, systems, rust-overlay, ... }:
    let eachSystem = nixpkgs.lib.genAttrs (import systems);
    in {
      devShells = eachSystem (system:
        let pkgs = import nixpkgs {
          overlays = [ (import rust-overlay) ];
          inherit system;
        };
        in {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rust-bin.nightly.latest.default
              pkg-config
              dbus
              udev 
            ];
          };
        });
    };
}
