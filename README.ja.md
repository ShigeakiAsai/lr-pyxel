# lr-pyxel

RetroArch/Lakka上で[Pyxel](https://github.com/kitao/pyxel)のゲームを動かすlibretroコアです。

[English README](README.md)

> **ステータス**：v0.8.4タグ付け済み、v0.9.0開発中（ドキュメント整備・リリース準備）。

---

## 概要

lr-pyxelは、CPython 3.11とヘッドレス版のPyxelエンジンをlibretroコアの中に埋め込み、
Pyxelのゲーム（`.py`/`.pyxapp`）をRaspberry Pi 5のようなデバイス上で
Lakka/RetroArch経由で動かせるようにするものです。

コンテンツの起動方法は2通りあり、それぞれ対応するファイル形式が異なります：

- **コンテンツありで起動する場合**（RetroArchのプレイリスト等から
  ファイルを直接コアのコンテンツとして読み込む場合）：**`.pyxapp`のみ**
  対応しています。`.pyxapp`は自己完結したパッケージ形式で、RetroArchの
  「コンテンツを直接読み込む」というモデルが想定している形にちょうど
  合致します——素の`.py`ファイルは、その意味で「一つのコンテンツ」として
  明確に定義された形ではありません。
- **コンテンツなしで起動する場合**：代わりに内蔵のランチャーが起動し、
  `/storage/roms/pyxel`を一覧表示します。**このランチャー内でのみ**、
  `.pyxapp`に加えて素の**`.py`**スクリプトも直接実行できます——
  ランチャーは単にフォルダの中身を一覧しているだけなので、両方とも
  同じように手軽に扱えます。

コア内蔵のダウンローダー（`downloader.pyxapp`）は、ランチャーのファイル一覧の
先頭にある「[Download new games]」から起動でき、同じフォルダへHTTP経由の
追加ゲーム取得ができます。取得したゲームは`/storage/roms/pyxel`に保存され、
ランチャーで選択することが可能です。

---

## 対応コンテンツ

| 形式 | コンテンツを直接指定して起動 | 内蔵ランチャー経由（コンテンツなし起動） |
|--------|:---:|:---:|
| `.pyxapp`（パッケージ済みアプリ） | ✅ | ✅ |
| `.py`（単体スクリプト） | ❌ | ✅ |

---

## ビルド方法

lr-pyxelはLakka/LibreELECのbuildrootチェックアウト内の1パッケージとしてビルドされ、
対象デバイス向けにクロスコンパイルされます（現時点ではRaspberry Pi 5 / aarch64を
対象に開発しています）。

```bash
# Lakka-LibreELECチェックアウトのルートディレクトリから：
DISTRO=Lakka PROJECT=RPi DEVICE=RPi5 ARCH=aarch64 scripts/clean pyxel
DISTRO=Lakka PROJECT=RPi DEVICE=RPi5 ARCH=aarch64 scripts/build pyxel
```

ビルドされたコアは、パッケージの`install_pkg`出力内の
`usr/lib/libretro/pyxel_libretro.so`に配置されます。

### 依存関係に関する注意

- `Cargo.toml`は`pyxel-core`を、本家Pyxelのフォークである
  [ShigeakiAsai/pyxel](https://github.com/ShigeakiAsai/pyxel)の
  **デフォルトブランチではなく`lr-pyxel`ブランチ**に固定しています——
  フォークの`main`ブランチは本家への貢献用にクリーンな状態を保っています
  （PR [kitao/pyxel#718](https://github.com/kitao/pyxel/pull/718)参照）。
  そちらのブランチへの変更を取り込んだ後は、再ビルド前に
  `cargo update -p pyxel-core`を実行してください。
- ネットワーク機能（`pyxel.download_file()` / `pyxel.http_get()`）は
  libcurlをリンクする代わりにシステムの`curl`バイナリを呼び出す実装なので、
  対象デバイスの`PATH`上に`curl`が必要です。
- Lakkaの組み込みPythonには、いくつかのコンパイル済み標準ライブラリ拡張
  （`_socket`、`_struct`、`_random`等）が欠けています。`math`・`random`・
  `struct`は起動時に`/tmp/lr-pyxel-stdlib`へ書き出されるPure Python版の
  スタブに置き換えられます。それ以外のコンパイル済み標準ライブラリ拡張が
  ABI不一致（`undefined symbol: ...`）で読み込めない場合があります——
  詳細は[既知の課題](#既知の課題)を参照してください。

---

## 既知の制限事項

以下のスクリプトパターンは、lr-pyxelのフレーム駆動型`retro_run()`モデルの
下では動作できません。v0.8.2以降、どちらも安全に失敗するようになっています
（RetroArchの画面通知を表示した上でランチャーへ戻ります。クラッシュやハングは
しません）。

- **`pyxel.flip()`を使うゲーム**（例：`99_flip_animation.py`）：
  `while True: ... pyxel.flip()`という定番のメインループパターンは、
  フロントエンドが毎フレーム`retro_run()`を1回呼ぶ（ゲーム側が自分で
  ループを回すのではない）というlibretroのモデルに合いません。
  `pyxel.flip()`は現在、黙って何もしない代わりに即座に例外を送出します
  （以前は無限ループがRustに一切制御を戻さないため、RetroArch全体が
  ハングしていました）。
- **`pyxel.cli` / アプリランチャー**（例：`17_app_launcher.py`）：
  Pyxel CLIおよびその独自のアプリ切り替え機構はヘッドレス環境では
  利用できません。`import pyxel.cli`は`ModuleNotFoundError`で失敗しますが、
  これは捕捉されてランチャーへ戻る形になります。
- **マウス入力**：未実装です——`retro_run()`は`RETRO_DEVICE_JOYPAD`のみを
  ポーリングし、`RETRO_DEVICE_MOUSE`は一切見ていないため、`mouse_x`/
  `mouse_y`が動くことはありません。`pyxel.mouse(True)`を呼んでも、
  動かない見せかけだけのカーソルを表示しないよう強制的に非表示にしています。
  v2.0.0で対応予定です。

---

## 既知の課題

- コンパイル済みのPython標準ライブラリ拡張の一部が、`undefined symbol: ...`
  で読み込みに失敗することがあります（Lakkaのシステム側Python 3.11ビルドと、
  lr-pyxelが埋め込むPyO3経由のPython 3.11ビルドとの間のABI不一致——
  同じバージョン番号でも内部のバイナリ構造が異なります）。現時点で確認済み：
  `_contextvars`（[sarananda.pyxapp](https://github.com/kadoyan/sarananda)
  がこれに該当し、起動自体ができません）。根本的な解決策は、システムの
  `lib-dynload`に一切触れない完全に自己完結した組み込みPython（例：
  [python-build-standalone](https://github.com/astral-sh/python-build-standalone)）
  への移行で、v2.0.0で対応予定です。
- バンク単位の音声・グラフィック状態（`sounds()`・`musics()`・`tones()`・
  `channels()`のgain/detune）は、コンテンツ切り替え時にリセットされません
  （パレット・画面サイズ・入力状態は既にリセットされるようになっています）。
  今のところ具体的な実害は確認されていませんが、同じ種類のバグが起こり得ます。
- `Tilemap.blt()`（トップレベルの`pyxel.bltm()`・`Tilemap`インスタンスメソッド
  の両方）は、整数のバンク番号のみを受け付け、`Tilemap`オブジェクトを
  ソースとして渡すことができません（`Image.blt()`はどちらも受け付けます）。

---

## 動作確認済みサンプル

実機（Raspberry Pi 5 / Lakka）で動作確認済みです：

- Pyxel公式サンプル：`01_hello_pyxel.py` 〜 `05_color_palette.py`、
  `07_snake.py`、`11_offscreen.py`、`15_tiled_map_file.py`
- `mega_wing.pyxapp`（公式サンプル）
- `30sec_of_daylight.pyxapp`（第1回Pyxel Jam優勝作品）
- `laser-jetman.pyxapp`
- `cursed_caverns.pyxapp`
- `vortexion.pyxapp`

---

## ライセンス

MIT

---

## クレジット

- [Pyxel](https://github.com/kitao/pyxel) by kitao
- [Lakka](https://www.lakka.tv/)
- [RetroArch](https://www.retroarch.com/)
