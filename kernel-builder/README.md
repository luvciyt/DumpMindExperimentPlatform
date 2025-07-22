 ## 1. 文件结构：

```
.
├── Cargo.lock
├── Cargo.toml
├── config
├── datasets
├── image   # 存放手动生成的镜像
├── README.md
├── src
├── target
├── template
└── workspace
```

其中 `image`

```
image
├── crash-bzImage   # 用于 kdump 的第二内核
├── crash-initramfs.cpio.gz # 用于 kdump 的用户空间镜像
└── debian.img # debian bullseye 用户空间镜像

1 directory, 3 files
```

> 用户可以通过编辑 crash-initramfs.cpio.gz 的 init.sh 控制启动行为。目前是自动将 vmcore 保存在 debian.img 下

`template`
```
template
└── shell.nix # 构建 linux kernel 编译的 nix-shell 环境

1 directory, 1 file
```
> 可以通过 `nix-shell -argstr compiler gcc-x` 来进入一个 gccx 编译环境的 nix-shell

## 2. 使用说明