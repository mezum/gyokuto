# TODO

仕様の詳細は [要件定義] を参照。上から順に実装する想定。

## MVP

- [ ] crate 構成 (ライブラリ + 動作確認用 CLI)
- [x] 字句解析
- [ ] CI (`just ci` で lint とテストを実行する GitHub Actions の workflow)
- [ ] 構文解析・AST
- [ ] 仕様書の語彙の統一
- [ ] エラー表示 (ariadne / miette)
- [ ] 名前解決・モジュール (`use` / `pub`、ホストによるソース解決)
- [ ] 型検査・局所型推論
- [ ] trait / impl
- [ ] ジェネリクスの単相化
- [ ] `dyn Trait` (vtable)
- [ ] move 検査・第二級参照の検査
- [ ] メモリレイアウト計算 (フラットな値型・フィールドオフセット)
- [ ] VM 方式の決定 (レジスタ型 / スタック型)
- [ ] バイトコード生成
- [ ] VM の基本命令・関数呼び出し
- [ ] 整数演算 (オーバーフロー時 panic)
- [ ] RAII (Drop の挿入、panic 時の Drop)
- [ ] `Result` / `Option` / `?` / panic
- [ ] クロージャ
- [ ] 標準ライブラリ (`Box` / `Vec` / `String` / `HashMap` / `Rc` / `Weak`)
- [ ] TypeInfo・実行時リフレクション・`dyn Any`
- [ ] comptime 評価
- [ ] ホスト関数の登録
- [ ] ホスト型の公開
- [ ] 実行制限 (fuel・メモリ上限)
- [ ] VM の `Send` 保証

## MVP 後

- [ ] バイトコードのファイル保存・バージョン管理・verifier
- [ ] ホットリロード
- [ ] コルーチン / `yield`
- [ ] serde 連携
- [ ] LSP
- [ ] デバッガ (プロトコルの決定を含む)
- [ ] REPL の検討

[要件定義]: ./docs/requirements.md
