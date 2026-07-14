# lr-pyxel クイックスタート（ビルド不要）

[Pyxel](https://github.com/kitao/pyxel)のゲームを、`Lakka`をインストールせず、
自分でビルドもせずに`RetroArch`で動かしたい方向けのガイドです。

## 1. ダウンロード

[Releasesページ](https://github.com/ShigeakiAsai/lr-pyxel/releases)から、
お使いのCPUに合ったファイルを取得してください（`Linux`専用です、下記の注意も
ご覧ください）。

- 一般的な`PC`・ノート`PC`（`Intel`/`AMD`系）：`pyxel_libretro-x86_64.so`
- `ARM`系（`Raspberry Pi`等）：`pyxel_libretro-aarch64.so`

> **注意**：これらは通常のデスクトップ／`ARM Linux`上で動く`RetroArch`向けの
> ビルドです。`Lakka`向けではありません。既に`Lakka`をお使いの場合は、
> 本体の[README](README.ja.md)をご覧ください。

## 2. リネームする

ダウンロードしたファイルを`pyxel_libretro.so`という名前に変更してください
（`-x86_64`/`-aarch64`の部分を外すだけです）。これは必須ではありません
（`RetroArch`は手動で選択すれば、どんなファイル名のコアでも読み込めます）が、
分かりやすくするためにお勧めします。

## 3. RetroArchのcoresフォルダにコピーする

置き場所は`Linux`のインストール方法によって異なります。

- **Flatpak版**：`~/.var/app/org.libretro.RetroArch/config/retroarch/cores/`
- **ネイティブパッケージ**：`~/.config/retroarch/cores/`や
  `/usr/lib/retroarch/cores/`であることが多いです。不明な場合は`RetroArch`の
  ディレクトリ設定をご確認ください

> `lr-pyxel`は`Linux`/`POSIX`系の`RetroArch`のみを対象としています
> （本体`README`の[既知の制限事項](README.ja.md#既知の制限事項)参照）——
> `Windows`向けビルドは存在せず、対応予定もありません。

## 4. Pyxelのゲームを起動する

`RetroArch`にて：**Load Core → `pyxel_libretro.so`を選択 → Load Content**
→ `.pyxapp`ファイルを選んでください。

手元に`.pyxapp`が無い場合は、公式の
[Pyxelサンプル](https://github.com/kitao/pyxel/tree/main/python/pyxel/examples)や、
コミュニティ作品を集めた[Pyxel User Examples](https://kitao.github.io/pyxel-user-examples/)
のギャラリーもお試しください。

## うまく動かない場合

まず[FAQ](FAQ.ja.md)をご確認ください（よくあるつまずきポイント——外部モジュール
不足、本家`Pyxel`では動くのに`lr-pyxel`では動かないケース等——をまとめています）。
それでも解決しない場合は、[Issue](https://github.com/ShigeakiAsai/lr-pyxel/issues)
を立てていただければと思います。
