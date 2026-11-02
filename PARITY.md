# Parity with Baritone 1.21.4

This port is **not complete**. This file lists the classes of Baritone 1.21.4 (`src/api` and `src/main`, 332 Java files) and what the Rust here does about each, so that nothing is claimed that is not true. It is also the work list for the rest.

| Status | Meaning |
|---|---|
| ported | written from the Java source and does the same thing; not yet diffed line by line against it |
| partial | exists, but simpler than the Java or differing in a way noted beside it |
| missing | not written |
| n/a | Java or Minecraft machinery with no meaning in Rust on Horizon (mixin accessors, the event bus, toasts) |

Rows below: 297 (classes that go together share a row). ported 78, partial 114, missing 66, n/a 39.

The settings are 243 of 252 (generated from Settings.java). The command layer (names, aliases, descriptions and messages) follows `baritone.command.defaults`; the processes and the path executor are the largest gap, and the Nether elytra pathfinder (22 files) is not started.


## api

| Java | Status | Rust or note |
|---|---|---|
| BaritoneAPI | n/a | one instance, no provider |
| IBaritone | partial | src/baritone.rs (Baritone, Core) |
| IBaritoneProvider | n/a | one instance |
| Settings | ported | src/api/settings.rs: 243 of 252, generated from Settings.java; not yet: buildSubstitutes, buildValidSubstitutes, buildRepeat, buildSchematicRotation, buildSchematicMirror (types the table cannot hold); the JavaOnly four are hidden by #set in the Java too |

## api/behavior

| Java | Status | Rust or note |
|---|---|---|
| IBehavior | n/a | Rust has no behavior registry |
| ILookBehavior | partial | src/behavior/look.rs: plain turning, no aim processors |
| IPathingBehavior | partial | src/behavior/pathing.rs |

## api/behavior/look

| Java | Status | Rust or note |
|---|---|---|
| IAimProcessor | missing | smooth aim with prediction is not ported |
| ITickableAimProcessor | missing | smooth aim with prediction is not ported |

## api/cache

| Java | Status | Rust or note |
|---|---|---|
| IBlockTypeAccess | missing |  |
| ICachedRegion | missing | no on-disk chunk cache |
| ICachedWorld | missing | no on-disk chunk cache |
| IWaypoint | ported | src/cache/waypoints.rs |
| IWaypointCollection | ported | src/cache/waypoints.rs |
| IWorldData | missing |  |
| IWorldProvider | missing |  |
| IWorldScanner | partial | src/game/world.rs (WorldCache): loaded chunks only |
| Waypoint | ported | src/cache/waypoints.rs |

## api/command

| Java | Status | Rust or note |
|---|---|---|
| Command | ported | src/command/manager.rs |
| IBaritoneChatControl | partial | src/baritone.rs (chat, command) |
| ICommand | n/a | a Command struct |
| ICommandSystem | n/a | registry() in manager.rs |

## api/command/argparser

| Java | Status | Rust or note |
|---|---|---|
| IArgParser | missing | typed parsers are methods of Args |
| IArgParserManager | missing | typed parsers are methods of Args |

## api/command/argument

| Java | Status | Rust or note |
|---|---|---|
| IArgConsumer | partial | src/command/argument.rs (Args) |
| ICommandArgument | partial | src/command/argument.rs |

## api/command/datatypes

| Java | Status | Rust or note |
|---|---|---|
| BlockById | partial | Core::block_named |
| EntityClassById | partial | follow: mob ids |
| ForAxis | ported | src/command/build.rs |
| ForBlockOptionalMeta | partial | names and [properties] only |
| ForDirection | ported | src/api/utils.rs |
| ForWaypoints | ported | src/cache/waypoints.rs (lookup) |
| IDatatype, IDatatypeContext, IDatatypeFor, IDatatypePost, IDatatypePostFunction | n/a | typed arguments are methods of Args |
| ItemById | partial | pickup validates an item id |
| NearbyPlayer | partial | follow player names |
| RelativeBlockPos | ported | src/command/manager.rs |
| RelativeCoordinate | ported | src/command/manager.rs |
| RelativeFile | partial | src/command/build.rs |
| RelativeGoal, RelativeGoalBlock, RelativeGoalXZ, RelativeGoalYLevel | ported | src/command/manager.rs (relative_goal) |

## api/command/exception

| Java | Status | Rust or note |
|---|---|---|
| CommandErrorMessageException, CommandException, CommandInvalidArgumentException, CommandInvalidStateException, CommandInvalidTypeException, CommandNoParserForTypeException, CommandNotEnoughArgumentsException, CommandNotFoundException, CommandTooManyArgumentsException, CommandUnhandledException, ICommandException | partial | one CommandError carrying the same messages |

## api/command/helpers

| Java | Status | Rust or note |
|---|---|---|
| Paginator | ported | src/command/argument.rs, manager.rs (paginate) |
| TabCompleteHelper | partial | src/command/tab.rs |

## api/command/manager and registry

| Java | Status | Rust or note |
|---|---|---|
| ICommandManager | partial | src/command/manager.rs |
| Registry | partial | registry() |

## api/event

| Java | Status | Rust or note |
|---|---|---|
| BlockChangeEvent | missing |  |
| BlockInteractEvent | missing |  |
| ChatEvent | ported | hooks/chat.rs |
| ChunkEvent | partial | WorldCache::note_chunks |
| PacketEvent | n/a | Horizon has no packet hook |
| PathEvent | missing |  |
| PlayerUpdateEvent | n/a | native_tick |
| RenderEvent | partial | hooks/lines.rs |
| RotationMoveEvent | n/a |  |
| SprintStateEvent | missing |  |
| TabCompleteEvent | ported | hooks/suggest_hook.rs |
| TickEvent | ported | hooks/tick.rs |
| WorldEvent | partial |  |
| Cancellable, EventState, ICancellable, Overrideable | n/a |  |
| AbstractGameEventListener, IEventBus, IGameEventListener | n/a | Horizon hook points instead |

## api/pathing

| Java | Status | Rust or note |
|---|---|---|
| IPath | ported | Path in src/pathing/calc/path.rs |
| IPathFinder | partial | AStar |
| IPathingControlManager | partial | Baritone::control_tick |
| Goal | ported | src/api/goals.rs |
| GoalAxis | ported |  |
| GoalBlock | ported |  |
| GoalComposite | ported |  |
| GoalGetToBlock | ported |  |
| GoalInverted | ported |  |
| GoalNear | ported |  |
| GoalRunAway | ported |  |
| GoalStrictDirection | ported |  |
| GoalTwoBlocks | ported |  |
| GoalXZ | ported |  |
| GoalYLevel | ported |  |
| ActionCosts | ported | src/pathing/costs.rs (with swimming and the 4097-entry fall table) |
| IMovement | partial |  |
| MovementStatus | partial |  |
| IPathExecutor | partial | src/pathing/path.rs |

## api/process

| Java | Status | Rust or note |
|---|---|---|
| IBaritoneProcess | ported | Process trait, src/behavior/pathing.rs |
| IBuilderProcess | partial | src/process/build.rs |
| ICustomGoalProcess | ported | src/process/goal.rs |
| IElytraProcess | partial | src/process/elytra.rs |
| IExploreProcess | partial | src/process/explore.rs |
| IFarmProcess | partial | src/process/farm.rs |
| IFollowProcess | partial | src/process/follow.rs |
| IGetToBlockProcess | partial | src/process/getto.rs |
| IMineProcess | ported | src/process/mine.rs |
| PathingCommand | ported | src/behavior/pathing.rs |
| PathingCommandType | ported | the variants of PathingCommand |

## api/schematic

| Java | Status | Rust or note |
|---|---|---|
| AbstractSchematic | partial |  |
| CompositeSchematic | ported | src/schematic/schematic.rs |
| CompositeSchematicEntry | ported |  |
| FillSchematic | ported |  |
| ISchematic | partial | Schematic trait: desired() instead of desiredState() |
| ISchematicSystem | partial | src/schematic/format.rs |
| IStaticSchematic | partial | StaticSchematic |
| MaskSchematic | partial | Shape |
| MirroredSchematic | missing |  |
| ReplaceSchematic | ported |  |
| RotatedSchematic | partial | rotate_quarter |
| ShellSchematic | partial | Shape::Shell |
| SubstituteSchematic | missing |  |
| WallsSchematic | partial | Shape::Walls |
| ISchematicFormat | partial |  |

## api/schematic/mask

| Java | Status | Rust or note |
|---|---|---|
| AbstractMask | missing | masks are replaced by Shape |
| Mask | missing | masks are replaced by Shape |
| PreComputedMask | missing | masks are replaced by Shape |
| StaticMask | missing | masks are replaced by Shape |
| BinaryOperatorMask | missing | masks are replaced by Shape |
| NotMask | missing | masks are replaced by Shape |

## api/schematic/mask/shape

| Java | Status | Rust or note |
|---|---|---|
| CylinderMask | partial | Shape::Cylinder, Shape::Sphere |
| SphereMask | partial | Shape::Cylinder, Shape::Sphere |

## api/selection

| Java | Status | Rust or note |
|---|---|---|
| ISelection | ported | src/selection/selection.rs |
| ISelectionManager | ported | src/selection/selection.rs |

## api/utils

| Java | Status | Rust or note |
|---|---|---|
| BetterBlockPos | partial | Pos = [i32; 3] |
| BlockOptionalMeta | partial | name and properties as text |
| BlockOptionalMetaLookup | partial | a set of names |
| BlockUtils | partial |  |
| BooleanBinaryOperator, BooleanBinaryOperators | missing |  |
| Helper | n/a | Core::say* |
| IInputOverrideHandler | partial | src/behavior/input.rs |
| IPlayerContext, IPlayerController | partial | Game |
| MyChunkPos, Pair, TypeUtils, VecUtils | n/a | Rust types |
| NotificationHelper, BaritoneToast | n/a | no desktop toast |
| PathCalculationResult | partial | Status |
| RayTraceUtils | partial | Game::clip_ray |
| Rotation, RotationUtils | partial | src/behavior/look.rs |
| SettingsUtil | partial | Settings::load |
| IItemStack, ILootTable | n/a | no loot tables |
| Input | partial | src/behavior/input.rs |
| IGoalRenderPos | partial | Goal::render_pos |

## main

| Java | Status | Rust or note |
|---|---|---|
| Baritone | partial | src/baritone.rs |
| BaritoneProvider, KeepName | n/a |  |

## main/behavior

| Java | Status | Rust or note |
|---|---|---|
| Behavior | n/a |  |
| InventoryBehavior | partial | src/behavior/inventory.rs: hotbar only |
| LookBehavior | partial | src/behavior/look.rs |
| PathingBehavior | partial | src/behavior/pathing.rs |
| WaypointBehavior | partial | death waypoint only |
| ForkableRandom | n/a |  |

## main/cache

| Java | Status | Rust or note |
|---|---|---|
| CacheFiles | missing |  |
| CachedChunk | missing |  |
| CachedRegion | missing |  |
| CachedWorld | missing |  |
| ChunkPacker | missing |  |
| FasterWorldScanner | partial | src/game/world.rs |
| WaypointCollection | ported | src/cache/waypoints.rs |
| WorldData | missing |  |
| WorldProvider | missing |  |
| WorldScanner | partial | src/game/world.rs |

## main/command

| Java | Status | Rust or note |
|---|---|---|
| CommandSystem | n/a |  |
| ExampleBaritoneControl | partial | src/baritone.rs (chat) |
| ArgParserManager | missing |  |
| DefaultArgParsers | missing |  |
| ArgConsumer | partial | src/command/argument.rs |
| CommandArgument | partial |  |
| CommandArguments | missing |  |
| CommandManager | partial | src/command/manager.rs |

## main/command/defaults

| Java | Status | Rust or note |
|---|---|---|
| AxisCommand | ported | nav.rs |
| BlacklistCommand | ported | work.rs |
| BuildCommand | partial | build.rs: own file reader, no Schematica |
| ClickCommand | partial | nav.rs: crosshair approximation, no GUI |
| ComeCommand | partial | nav.rs: goes to your feet, there is no camera |
| CommandAlias | ported | manager.rs |
| DefaultCommands | ported | manager.rs (ORDER) |
| ETACommand | ported | nav.rs |
| ElytraCommand | partial | work.rs: needs the Nether pathfinder for the same behaviour |
| ExecutionControlCommands | ported | nav.rs |
| ExploreCommand | partial | work.rs |
| ExploreFilterCommand | partial | work.rs |
| FarmCommand | partial | work.rs |
| FindCommand | ported | work.rs |
| FollowCommand | ported | work.rs |
| ForceCancelCommand | ported | nav.rs |
| GcCommand | partial | meta.rs: frees caches |
| GoalCommand | ported | nav.rs |
| GotoCommand | ported | nav.rs |
| HelpCommand | ported | meta.rs, manager.rs |
| InvertCommand | ported | nav.rs |
| LitematicaCommand | partial | build.rs: says Litematica is not present |
| MineCommand | ported | work.rs |
| PathCommand | ported | nav.rs |
| PickupCommand | ported | work.rs |
| ProcCommand | ported | nav.rs |
| ReloadAllCommand | partial | meta.rs: settings and waypoints, no chunk cache |
| RenderCommand | partial | meta.rs: prints Done |
| RepackCommand | partial | meta.rs |
| SaveAllCommand | partial | meta.rs |
| SchematicaCommand | missing | not in DefaultCommands of 1.21.4 |
| SelCommand | ported | build.rs |
| SetCommand | ported | meta.rs |
| SurfaceCommand | ported | nav.rs |
| ThisWayCommand | ported | nav.rs |
| TunnelCommand | ported | build.rs |
| VersionCommand | ported | meta.rs |
| WaypointsCommand | ported | meta.rs |

## main/event

| Java | Status | Rust or note |
|---|---|---|
| GameEventHandler | n/a | Horizon hook points, hooks/ |

## main/pathing/calc

| Java | Status | Rust or note |
|---|---|---|
| AStarPathFinder | ported | src/pathing/calc/a_star.rs: the Java search, resumable in slices; tested; not yet wired into the game |
| AbstractNodeCostSearch | ported | src/pathing/calc/a_star.rs: the seven coefficients and bestSoFar |
| Path | ported | src/pathing/calc/path.rs |
| PathNode | ported | src/pathing/calc/a_star.rs |
| BinaryHeapOpenSet | ported | src/pathing/calc/a_star.rs: same heap, same tie order |
| IOpenSet, LinkedListOpenSet | n/a |  |

## main/pathing/movement

| Java | Status | Rust or note |
|---|---|---|
| CalculationContext | ported | src/pathing/movement/calculation_context.rs |
| Movement | partial | data, costs and valid positions ported (movement.rs); updateState, swimming and aim are not |
| MovementHelper | partial | the cost half ported (movement_helper.rs); openDoors, attemptToPlaceABlock, moveTowards* are not |
| MovementOption | n/a |  |
| MovementState | partial | Controls, the older one |
| Moves | ported | src/pathing/movement/moves.rs |

## main/pathing/movement/movements

| Java | Status | Rust or note |
|---|---|---|
| MovementAscend | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |
| MovementDescend | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |
| MovementDiagonal | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |
| MovementDownward | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |
| MovementFall | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |
| MovementParkour | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |
| MovementPillar | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |
| MovementTraverse | partial | the cost is ported and tested (src/pathing/movement/movements/); the execution that walks it is still the older, simpler one |

## main/pathing/path

| Java | Status | Rust or note |
|---|---|---|
| CutoffPath | ported | src/pathing/calc/path.rs (cut_off) |
| PathExecutor | partial | src/pathing/path.rs |
| SplicedPath | missing | no path splicing |
| SprintJump | missing |  |

## main/pathing/precompute

| Java | Status | Rust or note |
|---|---|---|
| PrecomputedData | n/a | a speed-up of the Java |
| Ternary | n/a | a speed-up of the Java |

## main/process

| Java | Status | Rust or note |
|---|---|---|
| BackfillProcess | missing |  |
| BuilderProcess | partial | src/process/build.rs |
| CustomGoalProcess | ported | src/process/goal.rs |
| ElytraProcess | partial | src/process/elytra.rs: straight flight |
| ExploreProcess | partial | src/process/explore.rs |
| FarmProcess | partial | src/process/farm.rs |
| FollowProcess | partial | src/process/follow.rs |
| GetToBlockProcess | partial | src/process/getto.rs |
| InventoryPauserProcess | missing |  |
| MineProcess | ported | src/process/mine.rs: quantity from a drop table |

## main/process/elytra

| Java | Status | Rust or note |
|---|---|---|
| BlockStateOctreeInterface | missing | the Nether pathfinder |
| BuildLimitPathFinder | missing | the Nether pathfinder |
| ElytraBehavior | missing | the Nether pathfinder |
| IElytraPathFinder | missing | the Nether pathfinder |
| NetherPath | missing | the Nether pathfinder |
| NetherPathfinderContext | missing | the Nether pathfinder |
| PathCalculationException | missing | the Nether pathfinder |
| UnpackedSegment | missing | the Nether pathfinder |

## main/process/elytra/pathfinder

| Java | Status | Rust or note |
|---|---|---|
| BaritoneRegion | missing | the Nether pathfinder |
| BinaryHeapOpenSet | missing | the Nether pathfinder |
| Chunk | missing | the Nether pathfinder |
| ChunkGeneratorHell | missing | the Nether pathfinder |
| NetherPathfinder | missing | the Nether pathfinder |
| NodePos | missing | the Nether pathfinder |
| NoiseGeneratorImproved | missing | the Nether pathfinder |
| NoiseGeneratorOctaves | missing | the Nether pathfinder |
| PathFinder | missing | the Nether pathfinder |
| PathNode | missing | the Nether pathfinder |
| PathSegment | missing | the Nether pathfinder |
| Raytracer | missing | the Nether pathfinder |
| Size | missing | the Nether pathfinder |

## main/selection

| Java | Status | Rust or note |
|---|---|---|
| Selection | ported | src/selection/selection.rs |
| SelectionManager | ported | src/selection/selection.rs |
| SelectionRenderer | partial | src/game/render.rs |

## main/utils

| Java | Status | Rust or note |
|---|---|---|
| BaritoneMath | n/a | f64 methods |
| BaritoneProcessHelper | n/a |  |
| BlockBreakHelper | partial | behavior/pathing.rs (dig_step) |
| BlockPlaceHelper | partial | behavior/pathing.rs (click_block) |
| BlockStateInterface | partial | src/game/world.rs |
| BlockStateInterfaceAccessWrapper | n/a |  |
| GuiClick | missing |  |
| IRenderer | partial | src/game/render.rs |
| InputOverrideHandler | partial | src/behavior/input.rs |
| PathRenderer | partial | src/game/render.rs |
| PathingCommandContext | missing |  |
| PathingControlManager | partial | Baritone::control_tick |
| PlayerMovementInput | n/a | the game reads keys |
| ToolSet | partial | best_tool |

## main/utils/accessor

| Java | Status | Rust or note |
|---|---|---|
| IBlockStateFlags | n/a | Java mixin accessors |
| IChunkArray | n/a | Java mixin accessors |
| IChunkProviderClient | n/a | Java mixin accessors |
| IClientChunkProvider | n/a | Java mixin accessors |
| IEntityRenderManager | n/a | Java mixin accessors |
| IFireworkRocketEntity | n/a | Java mixin accessors |
| IGuiScreen | n/a | Java mixin accessors |
| IPalettedContainer | n/a | Java mixin accessors |
| IPlayerControllerMP | n/a | Java mixin accessors |

## main/utils/pathing and player

| Java | Status | Rust or note |
|---|---|---|
| Avoidance | missing |  |
| BetterWorldBorder | missing |  |
| Favoring | partial | a map of factors in the search; the previous-path part is not built yet, Avoidance is missing |
| MutableMoveResult | ported | src/pathing/movement/calculation_context.rs |
| PathBase | missing |  |
| PathingBlockType | partial | Cell |
| BaritonePlayerContext, BaritonePlayerController | partial |  |

## main/utils/schematic

| Java | Status | Rust or note |
|---|---|---|
| MapArtSchematic | missing |  |
| SchematicSystem | partial |  |
| SelectionSchematic | missing |  |
| StaticSchematic | partial |  |
| DefaultSchematicFormats | partial | src/schematic/format.rs |
| LitematicaSchematic | partial | .litematic read |
| MCEditSchematic | missing | old .schematic |
| SpongeSchematic | partial | .schem read |
| LitematicaHelper | n/a | needs the Litematica mod |
| SchematicAdapter, SchematicaHelper | n/a | need the Schematica mod |
| VarInt | partial | inside format.rs |
