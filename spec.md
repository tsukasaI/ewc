# ewc - Enhanced Word Count

## 概要

`ewc` は `wc` コマンドの改善版です。見やすい出力フォーマットとディレクトリの再帰処理をサポートします。

## インストール

```bash
cargo install ewc
```

## 基本使用例

### 単一ファイル

```bash
$ ewc file.txt
📄 file.txt
   Lines:      50
   Words:     200
   Bytes:   1,500
```

### 複数ファイル

```bash
$ ewc file1.txt file2.txt
📄 file1.txt
   Lines:      50
   Words:     200
   Bytes:   1,500

📄 file2.txt
   Lines:      30
   Words:     100
   Bytes:     800

─────────────────────────
📁 Total (2 files)
   Lines:      80
   Words:     300
   Bytes:   2,300
```

### ディレクトリ（サマリー）

```bash
$ ewc src/
📁 src/ (5 files)
   Lines:   1,234
   Words:   5,678
   Bytes:  45,000
```

### ディレクトリ（詳細）

```bash
$ ewc -v src/
📄 src/main.rs        45 lines
📄 src/lib.rs        123 lines
📄 src/utils.rs       67 lines
─────────────────────────────
📁 Total (3 files)   235 lines
```

## オプション

### Phase 1（優先実装）

| オプション | 短縮形 | 説明 |
|-----------|--------|------|
| `--lines` | `-l` | 行数のみ表示 |
| `--words` | `-w` | 単語数のみ表示 |
| `--bytes` | `-c` | バイト数のみ表示 |

#### 使用例

```bash
# 行数のみ
$ ewc -l file.txt
📄 file.txt
   Lines:      50

# 単語数のみ
$ ewc -w file.txt
📄 file.txt
   Words:     200

# 複数指定可能
$ ewc -lw file.txt
📄 file.txt
   Lines:      50
   Words:     200
```

### Phase 2（後から実装）

| オプション | 短縮形 | 説明 |
|-----------|--------|------|
| `--verbose` | `-v` | ファイル一覧を表示（ディレクトリ時） |
| `--all` | `-a` | 隠しファイル・ディレクトリを含める |
| `--compact` | `-C` | コンパクト出力（1行形式） |
| `--no-color` | - | 色・アイコンなしで出力 |
| `--json` | - | JSON形式で出力 |

#### 使用例

```bash
# 詳細表示
$ ewc -v src/
📄 src/main.rs        45 lines
📄 src/lib.rs        123 lines
📄 src/utils.rs       67 lines
─────────────────────────────
📁 Total (3 files)   235 lines

# 隠しファイルを含める
$ ewc -a src/

# コンパクト出力
$ ewc -C file.txt
file.txt: 50 lines, 200 words, 1,500 bytes

# 色なし出力
$ ewc --no-color file.txt
file.txt
   Lines:      50
   Words:     200
   Bytes:   1,500

# JSON出力
$ ewc --json file.txt
{"file":"file.txt","lines":50,"words":200,"bytes":1500}

# JSON + 複数ファイル
$ ewc --json file1.txt file2.txt
{"files":[{"file":"file1.txt","lines":50,"words":200,"bytes":1500},{"file":"file2.txt","lines":30,"words":100,"bytes":800}],"total":{"files":2,"lines":80,"words":300,"bytes":2300}}
```

## 挙動の詳細

### 隠しファイルの扱い

- **デフォルト**: `.` で始まるファイル・ディレクトリは除外
- **`-a` オプション**: 隠しファイル・ディレクトリを含める

```bash
$ ewc src/          # .gitignore, .hidden/ は除外
$ ewc -a src/       # 全て含める
```

### エラー処理

- 存在しないファイルはエラーメッセージを表示して続行
- 他のファイルは正常に処理
- 1つでもエラーがあれば終了コード 1

```bash
$ ewc nofile.txt existing.txt
⚠️  nofile.txt: No such file or directory

📄 existing.txt
   Lines:      50
   Words:     200
   Bytes:   1,500
```

### 標準入力

引数なしの場合は標準入力から読み込み（パイプ対応）。

```bash
$ cat file.txt | ewc
📄 <stdin>
   Lines:      50
   Words:     200
   Bytes:   1,500
```

## 出力フォーマット詳細

### 数値のフォーマット

- 3桁ごとにカンマ区切り
- 右揃え（6桁幅）

```
   Lines:      1,234
   Words:     12,345
   Bytes:    123,456
```

### アイコン

| アイコン | 意味 |
|---------|------|
| 📄 | ファイル |
| 📁 | ディレクトリ / Total |
| ⚠️ | エラー |

### 色

| 要素 | 色 |
|------|-----|
| ファイル名 | シアン |
| ラベル (Lines, Words, Bytes) | デフォルト |
| 数値 | 白/太字 |
| エラーメッセージ | 黄色 |

## 実装フェーズ

### Phase 1: 基本機能

1. 単一ファイルの行数・単語数・バイト数カウント
2. 見やすい出力フォーマット
3. `-l`, `-w`, `-c` オプション

### Phase 2: 複数ファイル対応

4. 複数ファイルの処理
5. Total の集計・表示

### Phase 3: ディレクトリ対応

6. ディレクトリの再帰処理
7. サマリー表示

### Phase 4: 追加オプション

8. `-v` 詳細表示
9. `-a` 隠しファイル対応
10. `-C` コンパクト出力
11. `--no-color` 色なし出力
12. `--json` JSON出力

### Phase 5: 仕上げ

13. 標準入力対応
14. エラーハンドリング強化
15. パフォーマンス最適化

## プロジェクト構成

```
ewc/
├── Cargo.toml
├── src/
│   ├── main.rs        # エントリーポイント
│   ├── lib.rs         # モジュール公開
│   ├── cli.rs         # CLIオプション定義 (clap)
│   ├── counter.rs     # カウントロジック
│   └── output.rs      # 出力フォーマット
└── tests/
    └── integration.rs # 統合テスト
```

## 依存クレート

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
walkdir = "2"
colored = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

## テスト方針

### ユニットテスト (counter.rs)

- 空ファイル → 全て0
- 1行のファイル → 行数1
- 複数行 → 正確なカウント
- 空白のみの行 → 行数にカウント、単語数0
- 日本語（マルチバイト）→ バイト数が文字数より多い

### 統合テスト (tests/)

- 単一ファイル処理
- 複数ファイル処理 + Total
- ディレクトリ処理
- 各オプションの動作
- 存在しないファイル → エラーメッセージ + 続行
- パイプ入力

## ライセンス

MIT
