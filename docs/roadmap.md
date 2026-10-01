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

### M1 特殊slot・Shift移動 実行記録

- slot.rsに挿入制約と回収除外を共有化し、pickup/drag/回収/UIの判断を接続。
- 醸造材料をJavaの実登録で6,912件比較。通常13種とraw/cooked fish metadata 3を一致確認。
- 醸造台・エンチャント台のShift移動を実装。複数stackを1個へ分割するNBT条件も再現。
- 公式サーバーで特殊slotへShift→pickup→通常inventoryへ戻す操作を検証し、NBTで12個/1個を確認。
- 未達: 金床/取引の取り出し条件、horse armor可否、クラフト/その他slot副作用、手動画面検証。
- M1は未完了。次はレシピと結果slotの材料消費を実装する。

### M1 クラフトエンジン 実行記録

- shaped/shapeless照合、位置ずらし・反転・metadata・重複材料を実装。99レシピを登録。
- 通常/右クリックの完成品スタック取得、材料1個消費、バケツ返却、結果再計算、Shift連続craft、拒否復元を接続。
- MCP CraftingManagerとの236ケース比較が一致。公式サーバーで2板材→4棒、6板材→12棒と材料消費を確認。
- 未達: 全レシピ登録、染色/花火/地図拡張/旗など残りの特殊レシピ、残余itemの全挙動、結果slotのthrow/swap、統計/実績、手動画面検証。
- 99レシピと限定統合検証はM1全体の完成ではない。次は全登録と特殊レシピを追加する。

### M1 修理・本/地図複製 実行記録

- 修理の耐久合算、最大耐久の5%ボーナス、0への制限、単品2個条件、NBT除去を実装。
- 本のNBT複製・generation更新・世代上限、原本1個の保持、空白本の消費を接続。
- 地図番号と表示名を引き継ぐ複製、空白地図と原本の消費を接続。
- modified UTF-8、型付きlist、compound、数値配列を保つサイズ制限付きNBT書き出しを追加。
- MCP919 Java比較: クラフト/修理2,036件、本/地図複製480件（NBT込み）が一致。
- 公式サーバーで修理後damage1261、地図2枚、複製本generation1と原本保持を確認。全workspaceテストも成功。
- 未達: 全登録、残りの特殊レシピ、複数stack原本のJava参照共有による残余挙動、統計/実績、手動画面検証。
- この記録はM1の部分実装であり、完全互換の宣言ではない。

### M1 革防具の染色 実行記録

- 革防具4種の染色を接続。既存色、染料のRGB、明度の平均・正規化をJava floatの演算順で再現。
- 耐久値、表示名、その他NBTを保持し、display.colorだけを更新。素材は各占有slotから1個消費。
- MCP919の8,550ケース（全16色、2染料混色、既存5色、範囲外metadata、非革防具）で出力/NBT一致。
- 公式サーバーで革胴着を赤に染め、色0x993333・耐久値17・材料消費を確認。
- 未達: 花火、旗、地図拡張、全静的レシピ、その他M1残作業。染色描画と手動操作の検証も残る。

### M1 花火クラフト 実行記録

- ロケット、花火の星、FadeColors追加をクラフト経路に接続。占有slotを数え、素材を各1個消費。
- 火薬1〜3slot、形状4種、Trail/Flicker、染料metadata & 15、星のExplosion順とNBT保持を再現。
- 星なしロケットはNBTなし。星ありではFlightとExplosionsを付け、欠けたExplosionは除外。
- MCP919の9,816ケースで出力とNBTが一致。不正素材・数量・形状、空/不正Explosionも比較。
- 公式1.8.9サーバーで星・ロケット・フェードの作成、結果slot取得、材料消費、transactionとInventory NBTを確認。
- 未達: 花火のentity描画・音・使用動作、旗、地図拡張、全静的レシピ、M1の残余/統計/GUI項目。

### M1 旗クラフト 実行記録

- 16色の旗の作成レシピを追加し、静的レシピ登録は115件へ拡張。
- banner.rsで全38模様の照合順、3x3染料配置、型紙item/metadata、染料なし型紙、模様6層上限を再現。
- BlockEntityTag/Patternsを追加し、元の耐久値・表示名・その他NBTを保持。
- 同じ基底色で模様あり/なし旗を複製。完成品1個を取得し、模様あり原本1個を残余として返却。
- MCP919の12,832ケースで出力/NBT一致。通常テストは原本返却、型紙だけの模様、不正list型、6層上限を検証。
- 公式サーバーで模様追加・複製、材料消費、原本保持、transaction/Inventory NBTを確認。全workspaceテスト成功。
- 未達: 旗の描画・設置動作、地図拡張、全静的レシピ、統計/実績、その他M1と後続マイルストーン。

### M1 静的レシピ全登録 実行記録

- MCP919の実登録を監査: 373登録中365件がShapedRecipes/ShapelessRecipes、8件が特殊レシピ。
- 部分的な115件の手作業登録を、365件のID/metadata/配置/出力/順序データへ置換。
- recipe_catalog.rsはレシピの事実データのみ。Javaソース・アセット・バイナリ・実行時Java依存を含めない。
- 材料の32767 wildcard、shapelessの厳密metadata、shapedの空cellを保持。静的/特殊レシピの選択は元の登録位置に従う（地図拡張の位置72は未実装）。
- 全365件を対象とした14,336ケースでJavaの出力と一致。2x2/3x3、offset、反転、shapeless配置、wildcard値、材料変更を比較。
- 既存の修理/複製/染色/花火/旗も含め、Java比較48,050ケースが成功。
- 公式サーバーで花崗岩1個・安山岩2個の作成、metadata、材料消費とInventory NBTを確認。
- 未達: 地図拡張（world mapデータ依存）、結果slotのthrow/swap、統計/実績、残余item細部、手動GUI検証。
- 静的レシピの全登録は、M1全体や完全互換の達成を意味しない。
