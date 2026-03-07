# GTO Exploit Solver — クラウド型ポーカーGTOソルバー 技術設計書

**作成日**: 2026年3月8日
**対象**: Claude Code実装用

---

## 1. プロダクト概要

### 1.1 コンセプト

「目の前の相手をどう倒すか」を教えるクラウド型ポーカーGTOソルバー。

GTO Wizardが「GTO的な正解を教える辞書」であるのに対し、本ツールは「実戦の特定の相手に対する最適なエクスプロイト戦略を算出する計算機」。

### 1.2 ターゲットユーザー

- GTO Wizardで基礎を学んだ中上級者
- PioSOLVERを使いたいが、高スペックPCやセットアップの手間を避けたい層
- 実戦の相手に合わせた戦略調整を求めるキャッシュゲームプレイヤー

### 1.3 MVP機能スコープ

| 機能 | 対応 | 補足 |
|------|------|------|
| カスタムレンジsolve | MVP | 自分・相手の任意レンジを入力してsolve |
| レーキ設定 | MVP | レーキ率 + キャップを指定可能 |
| ノードロック | MVP | ストリート間連鎖対応（ターンのロック→フロップに反映） |
| 集合分析 | MVP | 複数フロップ一括solve + CSV出力 |
| プリフロップsolve | 対象外 | |
| マルチウェイ | 対象外 | |
| 事前計算済みライブラリ | 対象外 | |
| LLM解説 | 対象外 | Phase 2以降検討 |
| ハンドヒストリー連携 | 対象外 | Phase 2以降検討 |

### 1.4 競合差分マトリクス

| 機能 | GTO Wizard | DeepSolver | PioSOLVER | 本ツール |
|------|-----------|------------|-----------|---------|
| カスタムレンジ | × | ○ | ○ | ○ |
| ノードロック | × | △（連鎖なし） | ○ | ○ |
| ストリート間連鎖ノードロック | × | × | ○ | ○ |
| レーキ設定 | ○ | × | ○ | ○ |
| 集合分析 | ○（固定条件） | ○（400フロップ上限） | ○ | ○ |
| CSV出力 | × | ○ | ○ | ○ |
| クラウド（ブラウザ完結） | ○ | ○ | × | ○ |
| カスタムフィルタリング | △ | △ | ○ | ○ |

---

## 2. システムアーキテクチャ

### 2.1 全体構成

```
┌──────────────────────────────────────────────────────────┐
│                     Frontend (React)                      │
│  ┌─────────┐  ┌──────────┐  ┌───────────┐  ┌──────────┐ │
│  │ Range    │  │ Board &  │  │ Node Lock │  │ Aggregate│ │
│  │ Editor   │  │ Tree UI  │  │ UI        │  │ Analysis │ │
│  └─────────┘  └──────────┘  └───────────┘  └──────────┘ │
└─────────────────────┬────────────────────────────────────┘
                      │ HTTPS / WebSocket
┌─────────────────────▼────────────────────────────────────┐
│                   API Server (Node.js)                    │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐              │
│  │ Auth &   │  │ Job      │  │ Result    │              │
│  │ Billing  │  │ Queue    │  │ Delivery  │              │
│  └──────────┘  └──────────┘  └───────────┘              │
└────────┬──────────┬──────────────┬───────────────────────┘
         │          │              │
    ┌────▼───┐ ┌────▼────┐  ┌─────▼─────┐
    │Stripe  │ │Redis    │  │PostgreSQL │
    │        │ │(Queue)  │  │(Results)  │
    └────────┘ └────┬────┘  └───────────┘
                    │
         ┌──────────▼──────────┐
         │  Worker Pool        │
         │  ┌────────────────┐ │
         │  │ CFR Engine     │ │
         │  │ (Rust binary)  │ │
         │  └────────────────┘ │
         │  Auto-scaling       │
         └─────────────────────┘
```

### 2.2 技術スタック

| レイヤー | 技術 | 選定理由 |
|---------|------|---------|
| CFRエンジン | **Rust** | C++並みの速度 + メモリ安全性。Claude Codeとの相性が良い |
| APIサーバー | **Node.js (Express or Fastify)** | フロントエンドとの親和性、WebSocket対応 |
| フロントエンド | **React + TypeScript** | GTO Wizard同等のリッチUI構築 |
| ジョブキュー | **Redis (BullMQ)** | 非同期solve管理、優先度制御 |
| データベース | **PostgreSQL** | ユーザー情報、solve結果保存 |
| 認証 | **Supabase Auth** or **Auth0** | ソーシャルログイン対応 |
| 決済 | **Stripe** | サブスク + solve上限管理 |
| インフラ | **AWS ECS Fargate** or **GCP Cloud Run** | Workerのオートスケーリング |
| ストレージ | **S3 / GCS** | 集合分析結果、CSV出力ファイル |

### 2.3 非同期処理フロー

```
1. ユーザーがsolveリクエストを送信
2. APIサーバーがsolve上限チェック（プラン別）
3. ジョブをRedis (BullMQ) キューに投入
4. 空いているWorkerがジョブを取得
5. CFRエンジン（Rustバイナリ）を実行
6. 結果をPostgreSQLに保存
7. WebSocketでユーザーに完了通知
8. ユーザーがブラウザで結果を閲覧（or CSV DL）
```

想定solve時間: 1〜5分（ツリーサイズ依存）

---

## 3. CFRエンジン仕様

### 3.1 アルゴリズム

**Discounted CFR (DCFR)** を採用。

CFR+よりも収束が速く、メモリ効率が良い。直近のイテレーションに重みを置くため、ノードロック後の再計算でも高速に収束する。

```
DCFR パラメータ:
- α = 1.5  (正のregretの割引)
- β = 0.0  (負のregretの割引)
- γ = 2.0  (戦略の割引)
```

### 3.2 ゲームツリー表現

```rust
// ゲームツリーのノード定義
enum Node {
    // チャンスノード（ボードカードの配布）
    Chance {
        children: Vec<(Card, NodeId)>,
    },
    // プレイヤーの意思決定ノード
    Action {
        player: Player,          // SB or BB
        info_set: InfoSetKey,    // 情報集合のキー
        actions: Vec<Action>,    // 取りうるアクション
        children: Vec<NodeId>,
        node_locked: bool,       // ノードロック状態
        locked_strategy: Option<Vec<f64>>,  // ロック時の固定戦略
    },
    // 終端ノード（ショーダウン or フォールド）
    Terminal {
        payoffs: [f64; 2],       // 各プレイヤーの利得
    },
}
```

### 3.3 情報集合 (Information Set)

```rust
// 情報集合のキー: ハンド + アクション履歴 + ボード
struct InfoSetKey {
    hand: [Card; 2],        // ホールカード
    board: Vec<Card>,       // コミュニティカード (0〜5枚)
    action_history: Vec<Action>,  // これまでのアクション列
}

// 情報集合に格納するデータ
struct InfoSetData {
    cumulative_regrets: Vec<f64>,    // 累積regret
    cumulative_strategy: Vec<f64>,   // 累積戦略
    current_strategy: Vec<f64>,      // 現在の戦略
}
```

### 3.4 ノードロック実装

```rust
// ノードロックの設定
struct NodeLock {
    node_id: NodeId,
    // ハンドごとのロック戦略（None = ロックなし）
    hand_strategies: HashMap<[Card; 2], Vec<f64>>,
}

// CFR走査時のノードロック処理
fn cfr_traverse(node: &Node, ...) -> f64 {
    match node {
        Node::Action { node_locked: true, locked_strategy, .. } => {
            // ロックされたノードでは戦略を更新せず、
            // 固定戦略に基づいてEVを計算
            let strategy = locked_strategy.unwrap();
            // 子ノードのEVを加重平均
            ...
        },
        Node::Action { node_locked: false, .. } => {
            // 通常のCFR更新
            ...
        },
        ...
    }
}
```

### 3.5 ストリート間連鎖ノードロック

PioSOLVERのみが対応し、DeepSolver/GTO Wizardが非対応の機能。

```
処理フロー:
1. ユーザーがTurnの特定ノードに対してノードロックを設定
2. Turn以降のサブツリーでCFRを実行し、ロック状態での均衡を計算
3. その結果をTurnの「到達確率」として固定
4. Flopからのフルツリーで再度CFRを実行
   → Turnの到達確率が変わっているため、Flopの戦略も変化
5. 収束するまで2〜4を反復（通常2〜3回で十分）
```

```rust
fn solve_with_chained_nodelock(
    game_tree: &GameTree,
    node_locks: &[NodeLock],  // ストリートごとのノードロック
    iterations: u32,
) -> Solution {
    // Phase 1: ロックされたストリート以降のサブツリーを先にsolve
    let downstream_solution = solve_subtree(
        game_tree,
        &node_locks,
        iterations,
    );

    // Phase 2: Phase 1の結果を反映して上流ストリートをsolve
    let full_solution = solve_full_tree(
        game_tree,
        &node_locks,
        &downstream_solution,
        iterations,
    );

    full_solution
}
```

### 3.6 レーキ計算

```rust
struct RakeConfig {
    percentage: f64,    // レーキ率 (例: 0.05 = 5%)
    cap: f64,           // レーキキャップ (例: 3.0 = 3BB)
    no_flop_no_drop: bool,  // フロップ前フォールドはレーキなし
}

// 終端ノードの利得計算にレーキを適用
fn calculate_terminal_payoff(
    pot: f64,
    winner: Player,
    rake_config: &RakeConfig,
    saw_flop: bool,
) -> [f64; 2] {
    let rake = if !saw_flop && rake_config.no_flop_no_drop {
        0.0
    } else {
        (pot * rake_config.percentage).min(rake_config.cap)
    };

    let net_pot = pot - rake;
    match winner {
        Player::SB => [net_pot / 2.0, -(net_pot / 2.0)],
        Player::BB => [-(net_pot / 2.0), net_pot / 2.0],
    }
}
```

### 3.7 ベットサイズ設定

```rust
// ユーザーが指定するベットサイズ構成
struct BetSizeConfig {
    flop: StreetBetSizes,
    turn: StreetBetSizes,
    river: StreetBetSizes,
}

struct StreetBetSizes {
    ip_bet: Vec<f64>,       // ポジションありのベットサイズ (ポット比率)
    oop_bet: Vec<f64>,      // OOPのベットサイズ
    ip_raise: Vec<f64>,     // IPのレイズサイズ
    oop_raise: Vec<f64>,    // OOPのレイズサイズ
    oop_donk: Vec<f64>,     // ドンクベットサイズ
}

// 例: 一般的な設定
let default_config = BetSizeConfig {
    flop: StreetBetSizes {
        ip_bet: vec![0.33, 0.67, 1.0],     // 33%, 67%, 100% pot
        oop_bet: vec![0.33, 0.67, 1.0],
        ip_raise: vec![2.5, 4.0],
        oop_raise: vec![2.5, 4.0],
        oop_donk: vec![0.33, 0.67],
    },
    turn: StreetBetSizes { ... },
    river: StreetBetSizes { ... },
};
```

### 3.8 カードの抽象化（イソモーフィズム）

計算速度の最適化のため、戦略的に等価なボードをまとめる。

```rust
// スートのイソモーフィズム
// 例: AhKhQs と AdKdQc は戦略的に等価
// フロップの1755通りのユニークボード（スート考慮後）を使用

fn canonicalize_board(board: &[Card]) -> CanonicalBoard {
    // スートのパーミュテーションで正規化
    // ♠→0, ♥→1, ♦→2, ♣→3 の順で最小表現に変換
    ...
}
```

### 3.9 収束判定

```rust
struct SolveConfig {
    max_iterations: u32,        // 最大イテレーション数 (デフォルト: 1000)
    target_exploitability: f64, // 目標exploitability (デフォルト: 0.3% pot)
    check_interval: u32,        // exploitability計算間隔 (デフォルト: 100)
    timeout_seconds: u32,       // タイムアウト (デフォルト: 300秒)
}

// 収束条件: 以下のいずれかを満たしたら終了
// 1. exploitability が target_exploitability 以下
// 2. max_iterations に到達
// 3. timeout_seconds を超過
```

### 3.10 CLI インターフェース

Rustバイナリは以下のJSON入力を受け取り、JSON出力を返す。

```json
// 入力 (stdin or ファイル)
{
    "job_id": "solve_abc123",
    "game": {
        "stack_size": 100.0,
        "pot_size": 6.5,
        "board": ["Qs", "8h", "4d"],
        "street": "flop",
        "players": {
            "oop": {
                "range": "AA,KK,QQ,JJ,TT,99,88,77,66,AKs,AQs,AJs..."
            },
            "ip": {
                "range": "AA,KK,QQ,JJ,TT,AKs,AKo,AQs..."
            }
        }
    },
    "bet_sizes": { ... },
    "rake": {
        "percentage": 0.05,
        "cap": 3.0,
        "no_flop_no_drop": true
    },
    "node_locks": [
        {
            "action_path": ["check", "bet:0.67", "call"],
            "street": "turn",
            "player": "ip",
            "hand_strategies": {
                "AhAs": [0.0, 1.0, 0.0],
                "KhKs": [0.5, 0.5, 0.0]
            }
        }
    ],
    "solve_config": {
        "max_iterations": 1000,
        "target_exploitability": 0.003,
        "timeout_seconds": 300
    }
}
```

```json
// 出力 (stdout or ファイル)
{
    "job_id": "solve_abc123",
    "status": "completed",
    "exploitability": 0.0028,
    "iterations": 742,
    "elapsed_seconds": 87.3,
    "solution": {
        "root": {
            "player": "oop",
            "actions": ["check", "bet:0.33", "bet:0.67"],
            "strategy": {
                "AhAs": [0.12, 0.45, 0.43],
                "AhKh": [0.67, 0.22, 0.11],
                ...
            },
            "ev": {
                "AhAs": 12.34,
                "AhKh": 8.76,
                ...
            },
            "children": { ... }
        }
    }
}
```

---

## 4. 集合分析仕様

### 4.1 概要

複数のフロップを一括でsolveし、フロップ横断の戦略傾向を分析する機能。

### 4.2 入力

```json
{
    "job_id": "aggregate_xyz789",
    "type": "aggregate_analysis",
    "game": {
        "stack_size": 100.0,
        "pot_size": 6.5,
        "oop_range": "...",
        "ip_range": "..."
    },
    "bet_sizes": { ... },
    "rake": { ... },
    "flop_filter": {
        "type": "all",           // "all" | "paired" | "monotone" | "rainbow" | "custom"
        "custom_flops": [],      // type=custom の場合
        "max_flops": 1755        // 上限
    },
    "solve_config": { ... }
}
```

### 4.3 出力（集合分析テーブル）

```json
{
    "job_id": "aggregate_xyz789",
    "total_flops": 1755,
    "completed_flops": 1755,
    "results": [
        {
            "board": ["Ah", "Kd", "7s"],
            "oop_ev": 2.34,
            "ip_ev": 4.16,
            "oop_equity": 0.47,
            "ip_equity": 0.53,
            "oop_eqr": 0.89,       // Equity Realization
            "ip_eqr": 1.11,
            "oop_actions": {
                "check": 0.62,      // 全ハンドの加重平均頻度
                "bet:0.33": 0.25,
                "bet:0.67": 0.13
            },
            "ip_actions": { ... },
            // フィルタリング用の特徴量
            "features": {
                "high_card": "A",
                "paired": false,
                "suited": false,     // monotone
                "connectivity": 0.2, // ボードのコネクト度
                "texture": "dry"     // dry / wet / neutral
            }
        },
        ...
    ]
}
```

### 4.4 CSV出力フォーマット

```csv
board,oop_ev,ip_ev,oop_equity,ip_equity,oop_eqr,ip_eqr,oop_check_freq,oop_bet33_freq,oop_bet67_freq,...
AhKd7s,2.34,4.16,0.47,0.53,0.89,1.11,0.62,0.25,0.13,...
QsJhTs,1.87,4.63,0.45,0.55,0.83,1.17,0.38,0.31,0.31,...
```

### 4.5 並列実行

集合分析は1755フロップを並列でsolveする必要がある。

```
- Worker 1台あたり 1 flop を処理
- BullMQ で 1755個のジョブを一括キューイング
- Auto-scaling で Worker を動的に増減
- 全フロップ完了後に集約してユーザーに通知
```

---

## 5. API設計

### 5.1 エンドポイント一覧

```
認証
POST   /api/auth/signup          - ユーザー登録
POST   /api/auth/login           - ログイン
POST   /api/auth/logout          - ログアウト

Solve
POST   /api/solve                - solveジョブ投入
GET    /api/solve/:jobId         - solve結果取得
GET    /api/solve/:jobId/status  - solveステータス確認
DELETE /api/solve/:jobId         - solveジョブキャンセル

集合分析
POST   /api/aggregate            - 集合分析ジョブ投入
GET    /api/aggregate/:jobId     - 集合分析結果取得
GET    /api/aggregate/:jobId/csv - CSV出力

ユーザー
GET    /api/user/profile         - プロフィール取得
GET    /api/user/usage           - 当月のsolve使用状況
GET    /api/user/history         - solve履歴一覧

課金
POST   /api/billing/subscribe    - プラン購読
POST   /api/billing/cancel       - プラン解約
GET    /api/billing/status       - 課金ステータス
POST   /api/billing/webhook      - Stripe Webhook
```

### 5.2 WebSocket イベント

```
// サーバー → クライアント
solve:progress    - solveの進捗 (イテレーション数, 現在のexploitability)
solve:completed   - solve完了
solve:failed      - solveエラー
aggregate:progress - 集合分析の進捗 (完了フロップ数 / 全フロップ数)
aggregate:completed - 集合分析完了
```

### 5.3 主要リクエスト/レスポンス例

```typescript
// POST /api/solve
interface SolveRequest {
    game: {
        stackSize: number;      // BBs
        potSize: number;        // BBs
        board: string[];        // e.g. ["Qs", "8h", "4d"]
        oopRange: string;       // e.g. "AA,KK,QQ,AKs,AKo..."
        ipRange: string;
    };
    betSizes: {
        flop: StreetBetSizes;
        turn: StreetBetSizes;
        river: StreetBetSizes;
    };
    rake?: {
        percentage: number;
        cap: number;
        noFlopNoDrop: boolean;
    };
    nodeLocks?: NodeLock[];
    solveConfig?: {
        maxIterations?: number;       // default: 1000
        targetExploitability?: number; // default: 0.003
        timeoutSeconds?: number;       // default: 300
    };
}

interface SolveResponse {
    jobId: string;
    status: "queued" | "running" | "completed" | "failed";
    estimatedSeconds?: number;
    queuePosition?: number;
}
```

---

## 6. フロントエンド設計

### 6.1 画面構成

```
/                    - LP（ランディングページ）
/login               - ログイン
/signup              - ユーザー登録
/app                 - メインダッシュボード
/app/solve           - 新規solve作成
/app/solve/:id       - solve結果表示
/app/aggregate       - 集合分析作成
/app/aggregate/:id   - 集合分析結果表示
/app/history         - solve履歴
/app/settings        - 設定・プラン管理
```

### 6.2 主要UIコンポーネント

#### レンジエディタ
13×13のハンドマトリクス。各セルをクリック/ドラッグで選択。

```
     A    K    Q    J    T    9    8    7    6    5    4    3    2
A  [AA] [AKs][AQs][AJs][ATs][A9s][A8s][A7s][A6s][A5s][A4s][A3s][A2s]
K  [AKo][KK] [KQs][KJs][KTs][K9s][K8s][K7s][K6s][K5s][K4s][K3s][K2s]
Q  [AQo][KQo][QQ] [QJs][QTs][Q9s][Q8s][Q7s][Q6s][Q5s][Q4s][Q3s][Q2s]
...
```

機能:
- セルクリックで100%選択 / 0%選択のトグル
- セル内で%スライダー（25%刻み → 1%刻み切替可能）
- テキスト入力モード（PioSOLVER互換のレンジ文字列）
- プリセットレンジの読み込み（UTG, MP, CO, BTN, SB, BB × RFI/3bet/call）
- GTO Wizardからのレンジコピペ対応

#### ボード設定
カードピッカーUI。Flop/Turn/Riverの各カードを選択。

#### ベットサイズ設定
ストリートごとに、IP/OOP × ベット/レイズのサイズをポット比率で入力。
プリセット（Small/Medium/Large）あり。

#### ノードロックUI
ゲームツリーをビジュアルに表示し、任意のノードをクリックしてロック。
ロック時はハンドごとの戦略を編集可能（レンジエディタと同様のUI）。

#### 結果表示
- レンジ全体のアクション頻度（カラーコード付きハンドマトリクス）
- 各ハンドのEV表示
- アクション別のレンジ分解
- ツリーナビゲーション（ノードをクリックして下層に移動）

#### 集合分析表示
- フロップ一覧テーブル（ソート・フィルタ対応）
- アクション頻度/EV/EQ/EQRでのソート
- テクスチャフィルター（ペア/モノトーン/レインボー等）
- CSV出力ボタン

### 6.3 技術要件

```
- React 18 + TypeScript
- 状態管理: Zustand or Jotai
- スタイリング: Tailwind CSS
- チャート: Recharts or D3.js（集合分析の可視化）
- WebSocket: socket.io-client
- ルーティング: React Router v6
```

---

## 7. データベース設計

### 7.1 テーブル定義

```sql
-- ユーザー
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255),
    stripe_customer_id VARCHAR(255),
    plan VARCHAR(50) DEFAULT 'free',  -- free, starter, pro, unlimited
    solve_limit INTEGER DEFAULT 0,
    solves_used_this_month INTEGER DEFAULT 0,
    billing_cycle_start TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Solveジョブ
CREATE TABLE solve_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    type VARCHAR(20) NOT NULL,  -- 'single' | 'aggregate'
    status VARCHAR(20) DEFAULT 'queued',  -- queued, running, completed, failed
    input JSONB NOT NULL,           -- SolveRequest全体
    result JSONB,                   -- solve結果
    exploitability FLOAT,
    iterations INTEGER,
    elapsed_seconds FLOAT,
    error_message TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP
);

-- 集合分析ジョブ
CREATE TABLE aggregate_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    status VARCHAR(20) DEFAULT 'queued',
    input JSONB NOT NULL,
    total_flops INTEGER,
    completed_flops INTEGER DEFAULT 0,
    result JSONB,                   -- 集約結果
    created_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP
);

-- 集合分析の個別フロップ結果
CREATE TABLE aggregate_flop_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_job_id UUID REFERENCES aggregate_jobs(id),
    board VARCHAR(10) NOT NULL,     -- e.g. "AhKd7s"
    result JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- レンジプリセット（ユーザー保存用）
CREATE TABLE saved_ranges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    name VARCHAR(255) NOT NULL,
    range_string TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);
```

---

## 8. 課金設計

### 8.1 プラン構成

| プラン | 月額 | solve上限/月 | 集合分析 | 主な対象 |
|--------|------|-------------|---------|---------|
| Free | $0 | 10回 | × | 試用 |
| Starter | $29 | 100回 | ×（Phase 2で追加） | ライトユーザー |
| Pro | $69 | 500回 | ○（月5回） | 中級者 |
| Unlimited | $149 | 無制限 | ○（無制限） | ハイステークスプレイヤー |

集合分析は1回 = 1ジョブ（最大1755フロップ）で、solve上限とは別カウント。

### 8.2 Stripe連携

```typescript
// プラン作成（Stripe Product/Price）
const plans = {
    starter: {
        priceId: 'price_starter_monthly',
        amount: 2900,  // $29
        solveLimit: 100,
    },
    pro: {
        priceId: 'price_pro_monthly',
        amount: 6900,  // $69
        solveLimit: 500,
        aggregateLimit: 5,
    },
    unlimited: {
        priceId: 'price_unlimited_monthly',
        amount: 14900,  // $149
        solveLimit: -1,  // unlimited
        aggregateLimit: -1,
    },
};

// Webhook でサブスク状態を管理
// invoice.paid → solve_used_this_monthリセット + plan更新
// customer.subscription.deleted → planをfreeに変更
```

### 8.3 solve上限の管理

```typescript
async function checkSolveLimit(userId: string): Promise<boolean> {
    const user = await db.users.findById(userId);
    if (user.plan === 'unlimited') return true;
    return user.solves_used_this_month < user.solve_limit;
}

async function incrementSolveCount(userId: string): Promise<void> {
    await db.users.increment(userId, 'solves_used_this_month', 1);
}
```

---

## 9. インフラ構成

### 9.1 AWS構成（推奨）

```
┌─ CloudFront (CDN) ─── S3 (React SPA)
│
├─ ALB ─── ECS Fargate (API Server × 2)
│           ├── Redis (ElastiCache)
│           └── PostgreSQL (RDS)
│
└─ ECS Fargate (Worker Pool)
    ├── Worker 1 (CFR Engine)
    ├── Worker 2 (CFR Engine)
    ├── ...
    └── Auto Scaling (CPU使用率ベース)
```

### 9.2 Worker Auto Scaling

```yaml
# ECS Auto Scaling設定
ScalingPolicy:
  MinCapacity: 1
  MaxCapacity: 20
  TargetTrackingScaling:
    TargetValue: 70.0              # CPU使用率70%を維持
    ScaleInCooldown: 60            # スケールイン待機60秒
    ScaleOutCooldown: 30           # スケールアウト待機30秒

# Workerインスタンスタイプ
TaskDefinition:
  Cpu: 4096      # 4 vCPU
  Memory: 8192   # 8GB RAM
```

### 9.3 コスト見積もり（月間）

| 項目 | 1000ユーザー想定 | 備考 |
|------|----------------|------|
| ECS API Server | ~$100 | 2タスク常時稼働 |
| ECS Worker | ~$200-800 | 平均2-8タスク（変動） |
| RDS PostgreSQL | ~$50 | db.t4g.medium |
| ElastiCache Redis | ~$30 | cache.t4g.micro |
| S3 + CloudFront | ~$20 | SPA + CSV出力 |
| **合計** | **~$400-1,000** | |

売上見積もり: 1000ユーザー × 平均$50 = $50,000/月

---

## 10. 開発ロードマップ

### Phase 1: CFRエンジン（2〜3週間）

```
1. Rust プロジェクト初期化
2. カード/デッキ/ハンド評価の基本データ構造
3. ゲームツリー生成（ベットサイズ設定対応）
4. DCFR アルゴリズム実装
5. レーキ計算
6. ノードロック（基本）
7. ストリート間連鎖ノードロック
8. JSON CLI インターフェース
9. テスト（PioSOLVERの出力と比較検証）
```

### Phase 2: バックエンド（1〜2週間）

```
1. Node.js API サーバー
2. PostgreSQL スキーマ + マイグレーション
3. BullMQ ジョブキュー
4. Worker プロセス（Rust バイナリ呼び出し）
5. WebSocket 通知
6. Stripe 連携（サブスク + Webhook）
7. 認証（Supabase Auth）
8. solve上限管理
```

### Phase 3: フロントエンド（2〜3週間）

```
1. React プロジェクト初期化
2. レンジエディタコンポーネント
3. ボード・ベットサイズ設定UI
4. solve投入 → 結果表示フロー
5. ノードロックUI
6. ゲームツリーナビゲーション
7. ダッシュボード（履歴、使用状況）
8. 課金・プラン管理画面
```

### Phase 4: 集合分析（1〜2週間）

```
1. 集合分析ジョブの一括キューイング
2. フロップフィルター
3. 結果テーブル（ソート・フィルタ）
4. CSV出力
5. Worker並列実行の最適化
```

### Phase 5: テスト・ローンチ準備（1週間）

```
1. E2Eテスト
2. 負荷テスト（同時solve数の確認）
3. LP作成
4. ドキュメント・使い方ガイド
5. ベータユーザー募集
```

**合計想定: 7〜11週間**

---

## 11. Claude Code への実装指示

### 11.1 開発順序

```
Step 1: Rust CFRエンジンを単体で完成させる
        → JSON入出力でテスト可能な状態にする
        → PioSOLVERの出力と比較して精度を検証

Step 2: Node.js APIサーバー + Redis + PostgreSQL
        → Docker Composeでローカル開発環境構築
        → Rust バイナリを Worker から呼び出す

Step 3: React フロントエンド
        → レンジエディタが最重要コンポーネント
        → GTO Wizardの UI を参考にするがコピーしない

Step 4: 全体結合 + Stripe連携 + デプロイ
```

### 11.2 各モジュールをClaude Codeに渡す際の注意

```
- CFRエンジン: このドキュメントのセクション3を丸ごと渡す
- API: セクション5のエンドポイント定義を渡す
- DB: セクション7のSQLをそのまま使う
- フロント: セクション6の画面構成とコンポーネント定義を渡す
- 各モジュールは独立してテスト可能に設計すること
```
