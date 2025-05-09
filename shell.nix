{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {

  packages = with pkgs; [
    # zlib
    pkg-config
    openssl
  ];

  # env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
  #   pkgs.stdenv.cc.cc.lib
  #   pkgs.zlib
  # ];

  shellHook = ''
    # exec fish
  '';

}

