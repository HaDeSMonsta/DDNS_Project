{
  description = "Flake to build the ddns project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
  let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in
  {
    devShells.${system}.default = pkgs.mkShell {
      packages = with pkgs; [
        pkg-config
        openssl
      ];
      shellHook = ''
        # Is this nice? No. Does it work? Yes!
        cd post_ip && cargo build --release
        exit
      '';
    };
  };
}

