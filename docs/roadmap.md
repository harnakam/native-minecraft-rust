# Minecraft 1.8.9 完全互換ロードマップ

## 完成条件

目的はMCP919を理解し、Minecraft 1.8.9のクライアント動作をRustで再実装すること。
現在は部分実装であり、既存の差分テストが成功しても完全互換とはしない。
Javaソースのディレクトリ対応は docs/mcp919-source-map.md で追跡する。
MCP、Minecraftのソース・JAR・画像・フォント・音声・ワールド・認証情報は公開しない。
必要なアセットは利用者のローカル環境から読み込む。

完成を宣言するには、以下の全領域について、実行経路が接続され、Javaとの挙動比較、
公式サーバーとの統合検証、実画面の操作検証が揃うことを要求する。
未対応をフォールバックで隠す、未対応パケットを捨てる、描画を箱に置換する、
テスト対象だけで動く実装は完了に数えない。

## 現在地

| 領域 | 現在の証拠 | 未達の主要項目 |
| --- | --- | --- |
| 接続 | ローカル公式サーバーの圧縮ログイン・継続通信 | 全パケットの効果、online-mode統合、切断・再接続 |
| 物理 | 旅行・衝突・選択・採掘の限定Java差分比較 | 全環境、騎乗、全エンティティ・効果・タイミング |
| インベントリ | pickup、swap、throw、限定shift、拒否ロールバック | drag、double-click、全slot制約、レシピ、特殊container |
| 描画 | 受信チャンクとローカルテクスチャを使用 | blockstate/model、透明面、照明、entity/item、particles |
| HUD | styled chat、Tab、title、一部ローカルフォント | 全字形、設定、装飾線、クリック・hover、完全な画面検証 |
| リソース | ローカルJARから限定アセットを読み込み | pack階層、reload、server pack、音声・言語・モデル |
| シングルプレイ | 実装なし | integrated server、生成、保存、全ゲーム処理 |

表は機能領域の監査であり、進捗率ではない。限定シナリオの成功を領域全体の完成に換算しない。
シングルプレイは従来の実装対象から外れていたが、ユーザーの「完全互換」という
最終目標では未達項目として残す。最後に黙って除外しない。

## 実装順序と受け入れ基準

### M0: 全体監査と実装責任の固定（着手）

- packet定義、decode、session適用、world/game適用、UI反映を照合し、未対応を列挙する。
- MCP919の呼び出し順と状態の所有者を source-map に対応させる。
- 各マイルストーンで既存の実行経路を使用し、空のJava風クラスを作らない。
- 完了基準: 全packet/画面/主要controller操作に実装場所と検証または未達理由がある。

### M1: インベントリとcontainer操作（次の主実装）

対象ソース: Container、ContainerPlayer、各Container、各Slot、GuiContainer、PlayerControllerMP。

1. drag mode 5: 開始・slot追加・終了、左右分配・creative分配、重複slot、数量不足、取消。
2. double-click mode 6: partial stack優先の二巡回収、NBT一致、方向、上限。
3. slotの挿入・取り出し・上限をcontainerごとに再現し、prediction/rollbackと共有する。
4. player/crafting tableのレシピ、結果slot、材料消費、残余アイテム、shift連続craft。
5. furnace、brewing、enchanting、anvil、merchant、beacon、hopper、dispenser、horse。
6. GUIのpress/move/releaseとruntime送信を接続し、閉じる・respawn・server更新で状態を解消する。

完了基準: MCP差分によるslot/cursor/NBT/return-stack/packet順の一致。
公式サーバーで左/右drag、double-click、craft、特殊container、transaction拒否・再同期を検証。
実画面で同じ操作が到達することを確認。純粋計算だけで完了にしない。

### M2: protocolとworld/entity lifecycle

対象ソース: NetworkManager、NetHandlerPlayClient、WorldClient、EntityTracker、各packet。

- 全clientbound/serverbound packetのwire形式と効果を網羅する。
- join、dimension変更、respawn、teleport、chunk unload、entity spawn/update/destroyの順序。
- metadata、attributes、effects、equipment、passenger/vehicle、velocity、explosion、teams、score。
- plugin message、統計、実績、設定、keepalive、切断理由・再接続を実際の状態に接続する。

完了基準: wire fixtures、Java state trace、公式サーバーのdimension/vehicle/death/reconnect検証。
未知または未対応のpacketを無視して互換性成功と判定しない。

### M3: リソースシステムと描画基盤

対象ソース: client/resources、TextureMap、BlockModelShapes、BlockRendererDispatcher、RenderGlobal。

- pack優先順位、ZIP/directory読込、asset index、モデルJSON、blockstate、親モデル、UV、rotation。
- texture atlas、animated texture、tint、透明/半透明、lightmap、ambient occlusion、空・雲・水。
- resource reloadをフォント・言語・モデル・音声まで伝播する。
- server packの同意設定、取得、hash/cache、status順序、成功・失敗・取消を実装する。

完了基準: Javaのモデル/UV/状態と比較し、代表的な全ブロック分類を実画面比較。
ローカル資源だけで動き、公開監査が通ること。cube置換や一律pack拒否では未完了。

### M4: entity/item/手持ち描画とゲームフィードバック

対象ソース: RenderManager、各Render/Model、ItemRenderer、EffectRenderer、SoundHandler。

- players、skins/capes、mobs、projectiles、dropped items、vehicles、装備と動作補間。
- first/third person、held-item、使用・弓・食事・採掘・攻撃・hurtのアニメーション。
- particles、block damage、音声、字幕ではなく1.8.9の音声動作。

完了基準: 各entity分類と視点の状態比較・画面比較・実操作。
境界箱のデバッグ描画を完成したentity描画と扱わない。

### M5: gameplay/physics/controllerの全状態

対象ソース: Entity、EntityLivingBase、EntityPlayerSP、PlayerControllerMP、各Block/Item。

- effects/attributes、全地形と流体、梯子、飛行、spectator、sprint、knockback、騎乗。
- 全block/item interaction、採掘例外、enchantment、creative/survival/adventure、reach。
- main-thread dispatch、tick/render補間、入力順序、packet emission順を原実装に合わせる。

完了基準: 複数tickのJava traceで位置・速度・flags・controller状態・送信packetが一致。
公式サーバーで各game modeと相互作用を検証。定数一致だけでは完了にしない。

### M6: 全画面・HUD・入力・設定

対象ソース: GuiScreen、GuiIngame、GuiNewChat、GuiPlayerTabOverlay、FontRenderer、GameSettings。

- menu、server list、auth/connect/disconnect、pause、options、controls、resource packs、各container。
- scoreboard、bossbar、Tab hearts、全font mapping/Unicode/UTF-16、bidi、GUI scale。
- chat scroll/history/completion、click/hover、clipboard、keybind、mouse感度、focus、fullscreen。
- 設定保存・読込、resource reload、resizeと画面遷移を実装する。

完了基準: 実画面の操作・focus/resize/keyboard・設定の再起動後保持。
フォントの限定glyph検証を全画面の見た目一致と扱わない。

### M7: singleplayer/integrated server

対象ソース: IntegratedServer、MinecraftServer、WorldServer、world generation/storage、game rules。

- Rustでworld生成・保存・読込、tick、entities、AI、recipes、redstone、network loopを実装。
- singleplayerの作成・選択・削除、LAN公開、pause、difficulty、game modeを接続する。

完了基準: Java版とのseed/生成/保存/ゲーム挙動比較、再起動後のworld保持、画面からの操作。
外部Javaプロセスで代用してRustネイティブ完成とはしない。現時点では未着手。

### M8: 完全互換の最終監査・配布

- M0–M7の未達項目をゼロにし、全検証を現在のHEADで再実行する。
- Windowsを含むサポート対象で起動、online-mode、長時間接続、再接続、performanceを検証。
- 意図するPvPサーバーの検証は互換性gateを通過した後に行う。
- asset/source/secret監査、再現可能build、操作方法と制約を確認し、harnakamへpushする。

完了基準: 各要件に現在の実装と直接の検証証拠があり、未実装/代替/mock/未検証がない。

## 実行ルール

- 一度の区切りは「機能の実装、UI/runtime接続、差分検証、統合検証、レビュー」。
  小さなフォント修正だけで全体進捗を報告する運用を終了する。
- 最初にM1をまとめて進める。内部変更を小さく保っても、報告は機能単位にまとめる。
- 検証環境の制限で実画面やonline-modeを確認できない場合は、そのgateを未達と明記する。
- 達成期限や進捗率は測定根拠なしに約束しない。完全再実装は大規模であり、
  ロードマップ作成や既存テスト成功だけでは完成ではない。
- docs/verification.md の履歴と照合し、各gateの証拠を追加する。

## M1 実行記録

- mode 6: player inventoryと通常storage、GUIの250ms検出とrelease、runtime、prediction/rollbackを接続。
- 公式サーバーで10+20+30を回収し、配置した60個をサーバー側NBTで確認。
- 未達: 手動mouse検証、Shift double-click、特殊containerルール。
- 次: mode 5 dragの状態機械とpress/move/release接続、共有slotルール、レシピ。
- M1は進行中。限定sliceの成功で完了とはしない。

### M1 drag 実行記録

- mode 5の開始・追加・終了、均等/1個/creative分配、数量不足・重複・上限・拒否復元を実装。
- press/move/releaseからruntimeの連続送信まで接続。creativeはmiddle dragを使用。
- 公式サーバー側NBTで、12個→4/4/4、12個→1/1/1とcursor残9、creative→64/64/64を確認。
- 未達: 手動mouse操作、preview、全特殊container/crafting副作用、他操作割込み時の厳密な順序。
- 次は共有slot/take/merge制約とレシピを進める。M1全体は未完了。
