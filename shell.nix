{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup
    clang
    btrfs-progs
  ];

  # Set environment variables for bindgen and linker
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  LD_LIBRARY_PATH = "${pkgs.llvmPackages.libclang.lib}/lib:${pkgs.btrfs-progs}/lib";

  RUST_LOG = "info";
  RUST_BACKTRACE = 1;

  PKG_CONFIG_PATH = "${pkgs.btrfs-progs}/lib/pkgconfig"; # Helps find libbtrfsutil

  shellHook = ''
    echo "Welcome to the Rust development environment for btrsnap"
    export RUSTUP_HOME=$HOME/.rustup
    export CARGO_HOME=$HOME/.cargo
    export PATH=$CARGO_HOME/bin:$PATH
    rustup default stable
    rustup component add rust-src  # For rust-analyzer source access.
    rustup component add clippy    # For linting.
    rustup component add rust-analyzer
  '';
}
