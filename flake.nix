{
  description = "btrsnap: A BTRFS snapshot manager for Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      eachSystem = lib.genAttrs lib.systems.flakeExposed;
      pkgsFor = nixpkgs.legacyPackages;
    in
    {
      packages = eachSystem (system: {
        default = pkgsFor.${system}.callPackage self { };
      });
      devShells = eachSystem (system: {
        default = pkgsFor.${system}.callPackage "${self}/shell.nix" { };
      });
      LIBCLANG_PATH = "${pkgsFor.llvmPackages.libclang.lib}/lib";
      BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgsFor.glibc.dev}/include -I${pkgsFor.btrfs-progs}/include";
      LD_LIBRARY_PATH = "${pkgsFor.llvmPackages.libclang.lib}/lib";
    };
}
