# Tetris DebugFS 调试接口说明

本项目为 `woc2026_hello_from_skm` 内核模块的俄罗斯方块游戏 (`/dev/tetris`) 提供了一组基于 `debugfs` 的带外（Out-of-band）调试和监控接口。

通过这一套接口，开发者可以在不干扰前台玩家（正在通过 `/dev/tetris` 进行游戏）的情况下，实时监控游戏的内部运行状态、读取统计数据、查看当前的随机包状态，甚至可以发送指令来直接干扰游戏进程。

## 1. 如何启用和挂载

当内核模块加载后，对应的 debugfs 节点会自动注册。首先，你需要确保 `debugfs` 已经挂载到了系统中：

```bash
# 挂载 debugfs (如果使用的是提供的 QEMU scripts/config-rootfs.sh 则已自动挂载)
mount -t debugfs none /sys/kernel/debug

# 进入 tetris_debugfs 目录
cd /sys/kernel/debug/tetris_debugfs/
```

## 2. 核心架构原理

为了让前台终端玩家的 `/dev/tetris` 游戏实例与后台的 `debugfs` 挂载点能够**共享同一个游戏状态**，我们重构了模块的数据所有权模型：

1. **全局单例迁移**：游戏核心 `TetrisGame` 不再由每个独占的文件描述符 `fd` 私有创建。我们利用 Rust for Linux (RFL) 的 `kernel::sync::global_lock!` 宏和 `Arc`（原子引用计数智能指针），在模块加载时(`module_init`) 初始化了一个全局的 `GLOBAL_DEVICE`。
2. **并发安全访问**：所有的游戏读取和修改，都被包裹在安全的互斥锁 `kernel::sync::Mutex` 下返回的 `Guard` 粒度控制中。无论是前台玩家按键还是后台触发读取，都严格串行化地访问游戏上下文，避免竞态崩溃。
3. **Debugfs 回调注入**：利用 RFL 构建 `read_callback_file` 以及双向的 `read_write_callback_file` 闭包（闭包中持有全局 `device` 的引用克隆），每当用户态进程去 `cat` 或 `echo` 节点文件时，系统自动调用这些闭包函数执行内部抓取或操纵逻辑。

## 3. 功能节点列表与使用说明

在 `/sys/kernel/debug/tetris_debugfs/` 目录下，共提供了以下 5 个伪文件节点。

### 3.1 `state` (只读 - 游戏当前状态)

**作用：** 快速了解当前游戏最核心的瞬时参数。
**使用：** `cat state`
**示例输出：**
```text
game_over:    false
score:        3800
current_type: T
current_x:    3
current_y:    14
current_rot:  2
next_type:    I
bag_idx:      3
```

### 3.2 `board` (只读 - ASCII 实况棋盘)

**作用：** 将内核态中的 `[[bool; 10]; 20]` 数据矩阵拼接结合当前下落方块，直接渲染成二维的 ASCII 连环画。非常适合配合 `watch` 命令作为双屏监视器！
**使用：** `cat board` （或 `watch -n 0.5 cat board` 持续监控）
**示例输出：**
```text
+--------------------+
|                    |
|                    |
|                    |
|        [][]        |
|      [][]          |
|                    |
|                    |
...
+--------------------+
```

### 3.3 `stats` (只读 - 对局数据统计)

**作用：** 满足进阶的数据分析。跟踪自游戏 `reset` 启动以来产生的消除和发牌概率情况。
**使用：** `cat stats`
**示例输出：**
```text
lines_total:    4
lines_single:   1
lines_double:   0
lines_triple:   1
lines_tetris:   0

pieces_total:   45
pieces_I:       6
pieces_O:       5
pieces_T:       7
...
```

### 3.4 `bag` (只读 - 随机数与 7-bag 系统透视)

**作用：** 展示核心机制 7-bag （防止连续拿到最差块的机制）当前的抽取状态，及底层线性同余发生器(LCG) PRNG 的内部种子值。你可以通过读取这里得知接下来 100% 会发什么牌。
**使用：** `cat bag`
**示例输出：**
```text
bag_idx:    3
remaining:  S J I O 
used:       T Z L 
prng_state: 0xd7b4e9f31a2b5c01
```

### 3.5 `control` (只写 - 游戏破坏/遥控器)

**作用：** 一个后门入口。通过给它抛特定的文本指令，可以直接跨站操纵核心循环机制。由于未导出原生 `write_only`，我们通过改写读写适配器来规避，实现只写逻辑。
**使用：** `echo <command> > control`
**支持的指令：**
- **控制方向**：`echo "left" > control` / `echo "right" > control` / `echo "down" > control` / `echo "rotate" > control`
- **硬降/重置**：`echo "drop" > control` / `echo "reset" > control`
- **强制步进**：`echo "tick" > control` (无需等待重绘即可直接让方块加速下落一格)
- **开挂发牌**：`echo "spawn <类型>" > control` (直接作废现有碎片并凭空刷出指定的方块类型！例如 `echo "spawn I" > control`)
