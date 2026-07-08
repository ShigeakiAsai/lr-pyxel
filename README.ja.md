# lr-pyxel

RetroArch/Lakka上で[Pyxel](https://github.com/kitao/pyxel)のゲームを動かすlibretroコアです。

[English README](README.md)

> **ステータス**：v0.11.0タグ付け済み、継続開発中。

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
  コンテンツ用フォルダ（「ROMS_DIR」、詳細は下記）を一覧表示します。
  **このランチャー内でのみ**、`.pyxapp`に加えて素の**`.py`**スクリプトも
  直接実行でき、さらに**サブフォルダへの移動**もできます（`[フォルダ名]`
  で中に入り、`..`で親に戻る）——ランチャーは単にフォルダの中身を
  一覧しているだけなので、ファイル形式もフォルダ階層も同じように
  手軽に扱えます。

`ROMS_DIR`自体は、ビルドの種類によって解決方法が異なります：
- **Lakkaビルド**（`lakka` Cargo feature）：`/storage/roms/pyxel`に
  固定されます。他のどのコアとも共通の`/storage/roms/<コンソール名>`
  という慣習に合わせているので、（Samba経由などで）ゲームを見つけやすく
  なっています。ランチャーはこのフォルダより上には移動できません。
- **非Lakkaビルド**：libretroの`RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY`
  呼び出しで実行時に決定されます（失敗時は`RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY`、
  さらにダメなら決め打ちのデフォルト値にフォールバック）。これはLakkaのような
  確立された慣習が無いためです。ここではランチャーはファイルシステム全体を
  移動でき、人為的な境界ではなくOSの権限に委ねる形になっています。

コア内蔵のダウンローダー（`downloader.pyxapp`）はコア本体のバイナリに
埋め込まれており、初回起動時に`{system_dir}/pyxel/downloader.pyxapp`
（`RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY`で解決される、`ROMS_DIR`＝
ユーザーコンテンツとは別の、コア自身の道具の置き場所）へ自動展開されます。
ランチャーのファイル一覧の先頭にある「[Download new games]」から起動でき
（`ROMS_DIR`直下でのみ表示され、サブフォルダ内では表示されません）、
`ROMS_DIR`へHTTP経由の追加ゲーム取得ができます。もし`ROMS_DIR`直下にも
`downloader.pyxapp`が存在する場合は、そちらが優先されます——これにより、
（将来の自己更新の仕組みなどを通じて）更新版のダウンローダーを`ROMS_DIR`に
置くだけで、コアの再ビルド・再デプロイ無しに即座に反映できます。

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
- `lakka` Cargo featureが、Lakka/LibreELEC固有のデフォルト値（[概要](#概要)参照）
  を切り替えます。**デフォルトでは無効**なので、Lakkaビルドは明示的に
  有効化する必要があります。`package.mk`が`cargo build`に`--features lakka`
  を渡しています。素の`cargo build`（一般的なLinux RetroArch向けなど）は
  非Lakka版のデフォルト値になります。
- ネットワーク機能（`pyxel.download_file()` / `pyxel.http_get()`）は
  libcurlをリンクする代わりにシステムの`curl`バイナリを呼び出す実装なので、
  対象デバイスの`PATH`上に`curl`が必要です。
- `retro_init()`は、埋め込みインタプリタを起動する前に、`libc`クレート経由で
  `libpython3.11.so`を`RTLD_GLOBAL`付きで再`dlopen`しています。これは
  RetroArchがこのコア（および、その依存先である`libpython3.11.so`）を
  `RTLD_GLOBAL`無しで読み込むため、そのライブラリのシンボルが、CPython自身が
  コンパイル済み拡張モジュールをimportする際にさらに行う`dlopen`呼び出しから
  見えなくなってしまうことへの対策です——「Pythonをプラグイン形式の共有
  ライブラリに埋め込む」際によく知られた落とし穴です。この対策が無いと、
  一部のコンパイル済み標準ライブラリ拡張が、`lr-pyxel`の外では問題なく
  動作するにも関わらず`undefined symbol: ...`で読み込みに失敗します。

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
  v0.12.x系で対応予定です。

---

## 既知の課題

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
- `sarananda.pyxapp`

---

## ライセンス

MIT

---

## クレジット

- [Pyxel](https://github.com/kitao/pyxel) by kitao
- [Lakka](https://www.lakka.tv/)
- [RetroArch](https://www.retroarch.com/)
