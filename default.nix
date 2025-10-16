{
  pkgs ? import <nixpkgs> { },
}:
pkgs.rustPlatform.buildRustPackage {
  pname = "btrsnap";
  version = "0.1.0";

  src = pkgs.lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  buildInputs = with pkgs; [
    btrfs-progs
  ];

  # # Ensure bindgen can find libclang and C headers
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.glibc.dev}/include";
  meta = with pkgs.lib; {
    description = "A command-line tool for managing BTRFS snapshots";
    homepage = "https://gitlab.com/0FGk3Zb2sY/btrsnap";
    license = licenses.mit;
    maintainers = [ "ks" ];
    platforms = platforms.linux;
  };
}
