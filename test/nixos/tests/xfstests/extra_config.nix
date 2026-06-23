{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs;
    [
      acl
      attr
      bc
      e2fsprogs
      exfatprogs
      file
      findutils
      fio
      gawk
      gnugrep
      gnused
      gnutar
      gzip
      inetutils
      keyutils
      libcap
      lvm2
      lz4
      openssl
      parted
      perl
      procps
      psmisc
      python3
      quota
      sqlite
      strace
      util-linux
      which
      xfsprogs
      xz
    ] ++ lib.optionals (pkgs ? xfstests) [
      xfstests
    ];
}
