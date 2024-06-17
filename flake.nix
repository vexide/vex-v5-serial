{
  description = "A Rust implementation of the V5 Serial Protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs = { nixpkgs, systems, ... }:
    let eachSystem = nixpkgs.lib.genAttrs (import systems);
    in {
      devShells = eachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in {
          default =
            pkgs.mkShell { buildInputs = with pkgs; [ pkg-config dbus udev ]; };
        });
    };
}
