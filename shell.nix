with import <nixpkgs> {};
let

  basePackages = [
    rustc
    cargo
    clang
    pkg-config
    libclang
    ipmitool
  ];

  inputs = basePackages;

  # define shell startup command
  hooks = ''

  '';

in mkShell {
  LIBCLANG_PATH="${libclang.lib}/lib";
  buildInputs = inputs;
  nativeBuildInputs = with pkgs; [ rustc cargo clang pkg-config libclang ];
  shellHook = hooks;
  RUST_SRC_PATH="${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}