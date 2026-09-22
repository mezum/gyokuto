# 構文仕様

[字句仕様] で得られたトークン列を入力とし、抽象構文木 (AST) を構築する。

## 記法

文法は以下の EBNF で記述する。

- `A ::= ...` は規則 `A` の定義
- `'x'` は記号・キーワードのトークン
- 大文字の名前はリテラル・識別子のトークン
  - `IDENT` `INT` `FLOAT` `CHAR` `STR` `RAW_STR` `BYTE` `BYTE_STR` `RAW_BYTE_STR`
- `A?` は省略可能、`A*` は 0 回以上、`A+` は 1 回以上の繰り返し
- `A | B` は選択、`( ... )` はグループ

## 位置とエラー

- AST の各ノードは、対応するトークン列の位置 (Span) を持つ
- 構文エラーがあっても解析を続け、複数のエラーをまとめて報告する
  - 括弧 `()` `[]` `{}` の内側でエラーが起きた場合は、対応する閉じ括弧まで読み飛ばして解析を続ける
- 字句解析のエラーとなったトークンは、構文解析ではエラーの位置として扱い、解析を続ける

## 式

```ebnf
Expr        ::= AssignExpr
PrimaryExpr ::= LiteralExpr | PathExpr | ParenExpr | TupleExpr | ArrayExpr
```

### リテラル式

```ebnf
LiteralExpr ::= INT | FLOAT | CHAR | STR | RAW_STR | BYTE | BYTE_STR | RAW_BYTE_STR
              | 'true' | 'false'
```

- 構文解析でリテラルを値に変換し、AST に保持する
  - 整数: `u64` の値とサフィックス。`u64` に収まらない値はどの整数型にも収まらないため、構文解析でエラーとする
  - 浮動小数点: `_` を除いた 10 進表記とサフィックス。`f32` / `f64` のどちらになるかは型検査で決まるため、値への変換は型検査で行う
  - 文字・バイト: `char` / `u8` の値
  - 文字列・バイト文字列: エスケープ・行継続・CRLF の正規化を処理した後の文字列 / バイト列
- 型ごとの値の範囲の検査は型検査で行う

### パス式

```ebnf
PathExpr    ::= PathSegment ( '::' PathSegment )*
PathSegment ::= IDENT | 'crate' | 'super' | 'self' | 'Self'
```

- `crate`・`self`・`Self` は先頭にのみ置ける
- `super` は先頭から連続する位置にのみ置ける (`super::super::a`)
- 先頭の `::` は使えない

### 括弧式・タプル式

```ebnf
ParenExpr ::= '(' Expr ')'
TupleExpr ::= '(' ')'
            | '(' Expr ',' ( Expr ( ',' Expr )* ','? )? ')'
```

- `()` はユニット、`(a,)` は要素 1 つのタプル、`(a)` は括弧式となる

### 配列式

```ebnf
ArrayExpr ::= '[' ( Expr ( ',' Expr )* ','? )? ']'
            | '[' Expr ';' Expr ']'
```

- `[a; n]` は `a` を `n` 個並べた配列となる

### 演算子式

```ebnf
AssignExpr ::= RangeExpr ( AssignOp AssignExpr )?
AssignOp   ::= '=' | '+=' | '-=' | '*=' | '/=' | '%=' | '&=' | '|=' | '^=' | '<<=' | '>>='
RangeExpr  ::= OrExpr ( ( '..' | '..=' ) OrExpr )?
             | OrExpr '..'
             | '..' OrExpr?
             | '..=' OrExpr
OrExpr     ::= AndExpr ( '||' AndExpr )*
AndExpr    ::= CmpExpr ( '&&' CmpExpr )*
CmpExpr    ::= BitOrExpr ( CmpOp BitOrExpr )?
CmpOp      ::= '==' | '!=' | '<' | '>' | '<=' | '>='
BitOrExpr  ::= BitXorExpr ( '|' BitXorExpr )*
BitXorExpr ::= BitAndExpr ( '^' BitAndExpr )*
BitAndExpr ::= ShiftExpr ( '&' ShiftExpr )*
ShiftExpr  ::= AddExpr ( ( '<<' | '>>' ) AddExpr )*
AddExpr    ::= MulExpr ( ( '+' | '-' ) MulExpr )*
MulExpr    ::= UnaryExpr ( ( '*' | '/' | '%' ) UnaryExpr )*
UnaryExpr  ::= ( '-' | '!' | '*' | '&' | '&' 'mut' ) UnaryExpr
             | PrimaryExpr
```

優先順位と結合性は以下の通り (上ほど強く結合する)。

| 演算子 | 結合性 |
| --- | --- |
| 単項 `-` `!` `*` `&` `&mut` | - |
| `*` `/` `%` | 左 |
| `+` `-` | 左 |
| `<<` `>>` | 左 |
| `&` | 左 |
| `^` | 左 |
| `\|` | 左 |
| `==` `!=` `<` `>` `<=` `>=` | なし |
| `&&` | 左 |
| `\|\|` | 左 |
| `..` `..=` | なし |
| `=` と複合代入 | 右 |

- 結合性が「なし」の演算子は連鎖できない
  - `a < b < c` や `a..b..c` は構文エラーとし、`(a < b) == c` のように括弧で明示する
- 単項 `-` は `-128i8` のようなリテラルにも演算子として適用する
  - 値の範囲の検査は型検査で行うため、`-128i8` は `i8` の最小値として扱える
- `&&` は 1 トークンの論理積であり、単項 `&` 2 つとしては扱わない
- 範囲式は端点を省略できる (`a..` `..b` `..` `..=b`)
  - `..=` は終端を省略できない
  - `..` の後に式が始まらないトークンが続く場合は、終端を省略したものとする
- 代入・複合代入は値が `()` の式とする
  - 右結合のため `a = b = c` は `a = (b = c)` と解析され、型検査で `()` の代入としてエラーとなる
  - 左辺が代入できる場所 (変数・フィールドなど) であるかは型検査で検査する

[字句仕様]: ./lexical.md
