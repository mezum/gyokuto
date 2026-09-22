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
PrimaryExpr ::= LiteralExpr | PathExpr | ParenExpr | TupleExpr | ArrayExpr | BlockExpr
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
PathExpr        ::= PathExprSegment ( '::' PathExprSegment )*
PathExprSegment ::= PathSegment ( '::' AngleArgs )?
PathSegment     ::= IDENT | 'crate' | 'super' | 'self' | 'Self'
```

- `crate`・`self`・`Self` は先頭にのみ置ける
- `super` は先頭から連続する位置にのみ置ける (`super::super::a`)
- 先頭の `::` は使えない
- 型引数は `::<` で始める (`Vec::<i32>::new`、`parse::<i32>`)
  - 式では `<` が比較演算子と紛らわしいため、型と違い `::` を必須とする

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

### ブロック式

```ebnf
BlockExpr ::= '{' Stmt* Expr? '}'
```

- 末尾の `;` を付けない式をブロックの値とする
  - 末尾の式が無い場合、ブロックの値は `()` となる
- ブロック内の `let` で導入した変数のスコープはブロックの終わりまでとする

### 後置式

```ebnf
PostfixExpr ::= PrimaryExpr PostfixOp*
PostfixOp   ::= '(' CallArgs? ')'
              | '.' IDENT ( '::' AngleArgs )? '(' CallArgs? ')'
              | '.' IDENT
              | '.' TupleIndex
              | '[' Expr ']'
              | '?'
CallArgs    ::= Expr ( ',' Expr )* ','?
TupleIndex  ::= INT
```

- `f(a, b)` は関数呼び出し、`a.f(b)` はメソッド呼び出しとなる
  - `a.f` の後に `(` が続く場合は常にメソッド呼び出しとする。フィールドの値を呼び出す場合は `(a.f)(b)` と書く
  - メソッドの型引数は `a.f::<T>(b)` と書く
- `a.name` は名前付きフィールド、`t.0` はタプルのフィールドへのアクセスとなる
  - タプルのフィールドは `_`・サフィックス・先頭の `0`・`0x` などを含まない 10 進数とする (`t.0` `t.12`)
  - `t.0.1` は字句としては `t` `.` `0.1` となるため、構文解析で浮動小数点のトークン `0.1` を `0` `.` `1` に分割する
    - 指数を含むもの (`t.0e1`) やサフィックスを持つものはエラーとする
- `a[i]` はインデックス、`a?` はエラー伝播となる

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
MulExpr    ::= CastExpr ( ( '*' | '/' | '%' ) CastExpr )*
CastExpr   ::= UnaryExpr ( 'as' TypeNoBounds )*
UnaryExpr  ::= ( '-' | '!' | '*' | '&' | '&' 'mut' ) UnaryExpr
             | PostfixExpr
```

優先順位と結合性は以下の通り (上ほど強く結合する)。

| 演算子 | 結合性 |
| --- | --- |
| 後置 (呼び出し・フィールド・メソッド呼び出し・インデックス・`?`) | - |
| 単項 `-` `!` `*` `&` `&mut` | - |
| `as` | 左 |
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
- `as` の後の型は `+` を含まない型 (TypeNoBounds) とする
  - 型の後の `<` は常に型引数の開始として扱う。比較する場合は `(x as usize) < y` のように括弧で囲む
- 範囲式は端点を省略できる (`a..` `..b` `..` `..=b`)
  - `..=` は終端を省略できない
  - `..` の後に式が始まらないトークンが続く場合は、終端を省略したものとする
- 代入・複合代入は値が `()` の式とする
  - 右結合のため `a = b = c` は `a = (b = c)` と解析され、型検査で `()` の代入としてエラーとなる
  - 左辺が代入できる場所 (変数・フィールドなど) であるかは型検査で検査する

## 文

```ebnf
Stmt          ::= ';' | LetStmt | ExprStmt
LetStmt       ::= 'let' 'mut'? IDENT ( ':' Type )? ( '=' Expr )? ';'
ExprStmt      ::= Expr ';'
                | BlockLikeExpr ';'?
BlockLikeExpr ::= BlockExpr
```

- `;` のみの文は何もしない
- `let` は変数を導入する
  - `mut` を付けた変数のみ再代入できる。再代入の検査は型検査で行う
  - 型を省略した場合は初期化の式から推論する
  - 初期化の式を省略した場合、使用前に必ず代入されているかは型検査で検査する
  - 同じ名前の `let` はそれまでの変数を隠す (shadowing)
- ブロック様の式 (BlockLikeExpr) は、文の先頭に置いた場合 `;` を省略できる
  - このとき式は文の終わりとなり、後に演算子を続けない。`{ a } - 1` は `{ a }` と `-1` の 2 つの文となる
  - `;` を省略した文の値が `()` であるかは型検査で検査する
  - ブロックの末尾に置いた場合は、ブロックの値となる

## 型

```ebnf
Type         ::= TypeNoBounds | 'dyn' TypeBounds | 'impl' TypeBounds
TypeNoBounds ::= PathType | RefType | ParenType | TupleType | ArrayType | SliceType
               | FnType | 'dyn' PathType | 'impl' PathType | '!' | '_'
TypeBounds   ::= PathType ( '+' PathType )*
```

- `dyn` / `impl` の後には `+` で複数のトレイトを書ける
  - 参照の対象や関数型の戻り値など、`+` が曖昧になる位置では括弧で囲む (`&(dyn A + B)`)
- `!` は値を返さない (発散する) ことを表す never 型とする
- `_` は型推論に任せる位置を表す (`Vec<_>`)
  - どこに書けるかは型検査で検査する

### パス型

```ebnf
PathType        ::= TypePathSegment ( '::' TypePathSegment )*
TypePathSegment ::= PathSegment GenericArgs?
GenericArgs     ::= AngleArgs
                  | '(' ( Type ( ',' Type )* ','? )? ')' ( '->' TypeNoBounds )?
AngleArgs       ::= '<' ( GenericArg ( ',' GenericArg )* ','? )? '>'
GenericArg      ::= Type | IDENT '=' Type | ConstArg | 'dyn' BlockExpr
ConstArg        ::= LiteralExpr | '-' LiteralExpr | BlockExpr
```

- パスの規則はパス式と同じ
- 型引数には型・関連型の指定 (`Iterator<Item = T>`)・定数 (const generics) を書ける
  - 定数はリテラル・負のリテラル・ブロック式 (`Buffer<{ N * 2 }>`) とする
  - `Buffer<N>` の `N` は構文上は型として解析し、型か定数かは名前解決で決める
- `dyn` のトレイトの型引数に限り、`dyn` を前置したブロック式で実行時に決まる値や型を書ける (`dyn Store<dyn { t }>`)
  - 型引数の `dyn` の後が `{` であるかで、`dyn Trait` と区別する
  - `dyn` のトレイト以外の型引数に書いた場合は型検査でエラーとする
- `Fn(A, B) -> C` のような括弧の型引数は、クロージャのトレイト `Fn` / `FnMut` / `FnOnce` に使う
- 型引数を閉じる位置にある `>>` `>=` `>>=` は、先頭の `>` を閉じ括弧とし、残りを次のトークンとして扱う
  - `Vec<Vec<T>>` は `>` 2 つとして解析する

### 参照型

```ebnf
RefType ::= '&' 'mut'? TypeNoBounds
```

- 参照は第二級であり、どこに書けるかは型検査で検査する
- ライフタイムの注釈は持たない
- `&&T` は参照の参照となるため書けない

### タプル型・括弧型

```ebnf
ParenType ::= '(' Type ')'
TupleType ::= '(' ')'
            | '(' Type ',' ( Type ( ',' Type )* ','? )? ')'
```

- `()` はユニット型、`(T,)` は要素 1 つのタプル型、`(T)` は括弧型となる

### 配列型・スライス型

```ebnf
ArrayType ::= '[' Type ';' Expr ']'
SliceType ::= '[' Type ']'
```

- `[T; N]` の `N` は定数として評価できる式とする
- スライス型は `&[T]` のように参照を通して使う

### 関数型

```ebnf
FnType ::= 'fn' '(' ( Type ( ',' Type )* ','? )? ')' ( '->' TypeNoBounds )?
```

- `->` を省略した場合、戻り値の型はユニット型となる

[字句仕様]: ./lexical.md
