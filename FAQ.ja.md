# lr-pyxel FAQ

[English FAQ](FAQ.md) | [README](README.ja.md)

---

## Q. ゲームが「Missing module: xxx」と表示されて起動しません

A. そのゲームが`pyxel`本体以外の外部Pythonモジュール（`numpy`等）に依存
しており、`lr-pyxel`側にそのモジュールが無いことを意味します。

`lr-pyxel`は`pyxel`単体としての動作を目指しており、外部モジュールの
サポートはスコープ外です。インストールするかどうかはユーザーの判断に
なります。

`Lakka`環境では`pip`が使えないため、`pip install`の代わりに、該当する
モジュールのwheelファイルを以下のいずれかの方法で配置することで動作
する場合があります。

- **SSH**：`/tmp/system/pyxel/site-packages/` へ展開（`/tmp`配下ですが、
  `RetroArch`の`System Directory`として永続化された場所なので、
  `RetroArch`再起動・システム再起動を挟んでも消えません）
- **ファイルブラウザ（Samba）**：`\\LAKKA\System\pyxel\site-packages`
  （`Windows`エクスプローラー等から。`Lakka`の設定でSambaが有効になって
  いる必要があります）

ただしこれは`lr-pyxel`としての動作保証がある機能ではなく、あくまで
参考情報としてご案内しています。

## Q. 特定のゲームが動きません。誰に報告すればいいですか？

A. まず、ネイティブの`pyxel`（`pip install pyxel`で構築した環境）で
同じ問題が再現するか確認してください。

- ネイティブの`pyxel`でも同じ問題が起きる → ゲームの作者様へご報告ください
- ネイティブの`pyxel`では問題なく、`lr-pyxel`でのみ発生する →
  [lr-pyxelのIssue](https://github.com/ShigeakiAsai/lr-pyxel/issues)
  へご報告ください

ネイティブの`pyxel`で動作する作品でも、`lr-pyxel`固有の制約
（外部モジュール非対応、セーブデータの永続化条件等——`README`の
[既知の制限事項](README.ja.md#既知の制限事項)参照）により動作しない
ケースがあることをご了承ください。
