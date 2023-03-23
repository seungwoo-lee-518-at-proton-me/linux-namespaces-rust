let
  unstable = import (fetchTarball https://nixos.org/channels/nixos-unstable/nixexprs.tar.xz) {};
	in
	{ pkgs ? import <nixpkgs> {} }:
	  pkgs.mkShell {
	    nativeBuildInputs = [
		    unstable.rustc
			  unstable.cargo
				unstable.rust-analyzer
				unstable.pkg-config
				unstable.openssl
		  ];
	  }
