# shell.nix: Universal kernel build environment supporting GCC and Clang versions
{ compiler ? "gcc-default" }:

let
  # 对于旧版本编译器，使用相应的旧 channel
  pkgs =
    if compiler == "gcc-8" then
      # 对于 gcc 8, 使用 nixos-21.05 channel
      import (builtins.fetchTarball {
        url = "https://github.com/NixOS/nixpkgs/archive/nixos-21.05.tar.gz";
      }) { }
    else if builtins.elem compiler [ "clang-8" "clang-9" "clang-10" "clang-11" ] then
      # 对于 Clang 8-11，使用 nixos-21.11 channel
      import (builtins.fetchTarball {
        url = "https://github.com/NixOS/nixpkgs/archive/nixos-21.11.tar.gz";
      }) { }
    else
      import <nixpkgs> { };

  # 内核构建通用依赖包
  commonKernelPkgs = with pkgs; [
    gnumake
    gdb
    git
    pkg-config
    bear
    perl
    python3
    ncurses
    cpio
    bc
    bison
    flex
    openssl
    openssl.dev
    elfutils
    elfutils.dev
    kmod
    rsync
    xz
    lz4
    zstd
    zlib
    zlib.dev
  ];

  # 解析 GCC 版本号
  parseGccVersion = compiler:
    if compiler == "gcc-default" then
      "default"
    else if pkgs.lib.strings.hasPrefix "gcc-" compiler then
      let
        versionStr = builtins.substring 4 (builtins.stringLength compiler) compiler;
      in
      if versionStr != "" then versionStr else "default"
    else
      "default";

  # 工具链配置
  toolchainConfig =
    if pkgs.lib.strings.hasPrefix "clang-" compiler then
      # Clang 配置
      let
        version = builtins.substring 6 (builtins.stringLength compiler) compiler;
        llvmAttr = "llvmPackages_" + version;
      in
      if builtins.hasAttr llvmAttr pkgs then
        let
          llvmPkgs = pkgs.${llvmAttr};
        in
        {
          stdenv = llvmPkgs.stdenv;
          packages = with llvmPkgs; [
            clang
            lld
            bintools
          ];
          hook = ''
            echo "   Toolchain: Clang (LLVM version ${version})"
            echo "   CC: $CC"
            echo "   CXX: $CXX"
            echo "   LD: ld.lld"
            ${if builtins.elem compiler [ "clang-8" "clang-9" "clang-10" "clang-11" ] then ''
              echo "       Channel: nixos-21.11 (for Clang ${version} support)"
              echo "       Note: Using nixos-21.11 channel for Clang ${version} with compatible dependencies"
            '' else ""}
          '';
          makeFlags = "LLVM=1 CC=clang LD=ld.lld AR=llvm-ar NM=llvm-nm OBJCOPY=llvm-objcopy STRIP=llvm-strip -j$(nproc)";
        }
      else
        throw "Error: Clang version '${version}' not found. Available versions: 8, 9, 10, 11 (from nixos-21.11), 12, 13, 14, 15, 16, 17, 18, 19"
    else
      # GCC 配置
      let
        gccVersionStr = parseGccVersion compiler;

        selectedGcc =
          if gccVersionStr == "default" then
            pkgs.gcc
          else if gccVersionStr == "8" then
            # GCC 8 特殊处理：从 nixos-20.09 channel 获取
            pkgs.gcc8
          else
            let
              gccAttr = "gcc${gccVersionStr}";
            in
            if builtins.hasAttr gccAttr pkgs then
              pkgs.${gccAttr}
            else
              throw "Error: GCC version '${gccVersionStr}' not found. Available versions: 8 (from nixos-24.05), 9, 10, 11, 12, 13, 14";

        # 使用选定的 GCC 创建 stdenv
        gccStdenv = pkgs.overrideCC pkgs.stdenv selectedGcc;
      in
      {
        stdenv = gccStdenv;
        packages = [
          selectedGcc
          pkgs.binutils
        ];
        hook = ''
          export CC=${toString selectedGcc}/bin/gcc
          export CXX=${toString selectedGcc}/bin/g++
          echo "   Toolchain: GCC (Version: ${gccVersionStr})"
          echo "   CC: $CC"
          echo "   CXX: $CXX"
          echo "   Binutils: ${pkgs.binutils}/bin"
          ${if gccVersionStr == "8" then ''
            echo "      Channel: nixos-24.05 (for GCC 8 support)"
            echo "      Note: Using nixos-24.05 channel for GCC 8 with updated dependencies"
          '' else ""}
        '';
        makeFlags = "-j$(nproc)";
      };

in
toolchainConfig.stdenv.mkDerivation {
  name = "kernel-build-env-${compiler}";

  buildInputs = toolchainConfig.packages ++ commonKernelPkgs;

  shellHook = ''
    ${toolchainConfig.hook}

    # 设置 elfutils 环境变量
    export ELFUTILS_LIB=${pkgs.elfutils.dev or pkgs.elfutils}
    export PKG_CONFIG_PATH="$ELFUTILS_LIB/lib/pkgconfig:$PKG_CONFIG_PATH"
    export HOSTCFLAGS="-I$ELFUTILS_LIB/include"
    export HOSTLDFLAGS="-L$ELFUTILS_LIB/lib -Wl,-rpath,$ELFUTILS_LIB/lib"

    # 设置内核构建相关环境变量
    export KBUILD_BUILD_HOST="nix-kernel-build"
    export KBUILD_BUILD_USER="$USER"

    echo "========================================================"
    echo "   Kernel Build Environment Ready!"
    echo ""
    echo "   Environment Info:"
    echo "   Selected compiler: ${compiler}"
    echo "   Build flags: ${toolchainConfig.makeFlags}"
    echo ""
    echo "  Compiler Details:"
    if [[ "${compiler}" == clang-* ]]; then
        clang --version | head -n1
        echo "   LLVM tools available: clang, lld, llvm-ar, llvm-nm, llvm-objcopy, llvm-strip"
    else
        gcc --version | head -n1
        echo "   Binutils version: $(ld --version | head -n1)"
    fi
    echo ""
    echo "   Usage Examples:"
    echo "   make ${toolchainConfig.makeFlags} defconfig"
    echo "   make ${toolchainConfig.makeFlags} menuconfig"
    echo "   make ${toolchainConfig.makeFlags} bzImage modules"
    echo "   make ${toolchainConfig.makeFlags} modules_install INSTALL_MOD_PATH=./install"
    echo ""
    echo "   Compiler Usage Examples:"
    echo "   nix-shell --arg compiler '\"gcc-8\"'        # GCC 8"
    echo "   nix-shell --arg compiler '\"clang-8\"'      # Clang 8"
    echo "   nix-shell --arg compiler '\"clang-11\"'     # Clang 11"
    echo "   nix-shell --arg compiler '\"clang-17\"'     # Clang 17"
    echo "========================================================"
  '';
}