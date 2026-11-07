/*! # 堆对象布局（浅堆口径的唯一真源）

浅堆（Shallow Heap）= 对象自身在堆上占的字节数，**不含**它引用的对象。这个数在
三条链路里都要用（直方图 / 支配树 retained / OQL `@shallow`），各处自己写一遍常量
必然漂移 —— 所以对象头、对齐、压缩指针这些参数只在本模块推导一次。

## 口径（P3-4 之后）

| 对象 | 浅堆 |
|---|---|
| 实例 | HPROF ClassDump 的 `instance_size`（已含对象头与全部继承字段），向上对齐且不小于实例对象头 |
| 对象数组 | 数组头 + 元素数 × 引用宽度，整体向上对齐 |
| 基本类型数组 | 数组头 + 元素数 × 元素宽度，整体向上对齐 |
| 类对象 | `java.lang.Class` 的 `instance_size`（类对象本身就是一个 Class 实例），缺失时退回实例对象头 |

**类对象不再按 0 计**：它是真实占用堆内存的对象，按 0 计会让支配树里
「类加载器 retain 了多少」系统性偏小。代价是与直方图的一处已知差异：直方图**不把
类对象单独列行**（与 Eclipse MAT 的 Class Histogram 一致，MAT 把类对象归到
ClassLoader 视图里），因此「支配树可达合计」会比「直方图合计」大一点点，
差值 ≈ 类对象数 × 类对象浅堆。这是有意的口径分工，不是算错。

## 对象头如何推导

HotSpot 对象布局（`oopDesc`）：

```text
实例:  [ mark word ][ klass pointer ]                      → 对齐
数组:  [ mark word ][ klass pointer ][ array length (4B) ] → 对齐
```

| 位数 / 指针模式 | mark | klass | 实例头 | 数组头 |
|---|---|---|---|---|
| 64 位 + 压缩指针（`-XX:+UseCompressedOops`，默认） | 8 | 4 | 16 | 16 |
| 64 位 + 普通指针（堆 > 32GB 或 `-XX:-UseCompressedOops`） | 8 | 8 | 16 | 24 |
| 32 位（引用即 4 字节） | 4 | 4 | 8 | 16 |

压缩指针在实例头上**看不出差别**（12 对齐到 16 ≡ 16），差异只体现在数组头（20 vs 16）。
HPROF 不记录 `UseCompressedOops`，所以判定规则是：

1. `id_width == 4` ⇒ 32 位 / 压缩引用；
2. `id_width == 8` ⇒ 默认按**压缩指针**（JDK 6u23 起、堆 < 32GB 的默认值，
   桌面与服务端都是绝大多数情况）；
3. 反例（大堆未压缩、`-XX:ObjectAlignmentInBytes=16` 等）用 `plugins/mat.json` 的
   `heapLayout.compressedOops` / `heapLayout.alignment` 覆盖 —— 这类 dump 的数组头会
   差 8 字节，出现在「数组占比高」的堆里足以影响结论。

**HPROF 的数组长度是显式字段，不用猜**；但很多实现（包括我们之前的版本）会漏掉
null 元素与图外引用 —— 它们同样占着槽位。本模块只负责「n 个元素多少字节」，
数错 n 是调用方的问题（`mat_graph` 已改为按元素总数而非「解析成功的引用数」计）。
*/

use crate::mat_cfg;

/// 默认对象对齐粒度（HotSpot `-XX:ObjectAlignmentInBytes` 默认 8）
pub(crate) const DEFAULT_ALIGN: u64 = 8;

/// 类对象（`java.lang.Class` 实例）的类名：HPROF 内部名用 `/`，少数工具用 `.`
const CLASS_OF_CLASS: [&str; 2] = ["java/lang/Class", "java.lang.Class"];

/// 堆布局参数（浅堆计算的输入；构建一次、随图一块走）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeapLayout {
    /// 引用宽度（4 / 8 字节）
    pub id_width: u8,
    /// 对象对齐粒度（4 / 8 / 16）
    pub align: u8,
    /// 是否使用压缩对象指针（只影响 klass 指针宽度 ⇒ 数组头）
    pub compressed_oops: bool,
    /// 对齐后的实例对象头（字节）
    pub instance_header: u32,
    /// 对齐后的数组对象头（含 4 字节 length）
    pub array_header: u32,
    /// 类对象浅堆（`java.lang.Class` 的 instance_size；0 = dump 里没有该类的类记录）
    pub class_shallow: u32,
}

impl HeapLayout {
    /// 按显式参数构造（单测与配置覆盖都走这里）
    pub(crate) fn with_params(id_width: u64, align: u64, compressed: Option<bool>) -> Self {
        let id_width = if id_width == 4 { 4u8 } else { 8u8 };
        let align = if align.is_power_of_two() && (4..=32).contains(&align) {
            align
        } else {
            DEFAULT_ALIGN
        };
        let compressed = compressed.unwrap_or(true);
        let mark = if id_width == 4 { 4u64 } else { 8 };
        let klass = if id_width == 4 || compressed { 4 } else { 8 };
        let instance_header = align_up(mark + klass, align);
        let array_header = align_up(mark + klass + 4, align);
        HeapLayout {
            id_width,
            align: align as u8,
            compressed_oops: compressed,
            instance_header: instance_header as u32,
            array_header: array_header as u32,
            // 兜底：类记录缺失时类对象至少占一个实例头；`with_class_size` 会用
            // dump 里 `java.lang.Class` 的真实 instance_size 覆盖它
            class_shallow: instance_header as u32,
        }
    }

    /// 按 dump 的引用宽度 + `plugins/mat.json` 的 `heapLayout` 覆盖构造。
    /// **每个入口只调一次**（它要读配置文件），不要在逐对象循环里调。
    pub(crate) fn from_id_width(id_width: u64) -> Self {
        let cfg = mat_cfg::layout_cfg();
        Self::with_params(id_width, cfg.alignment.unwrap_or(DEFAULT_ALIGN), cfg.compressed_oops)
    }

    /// 从索引头还原布局（字段非法 ⇒ None，调用方整份判废）。
    /// 索引里存的是**绝对字节数**而不是重新推导的参数：导出索引的机器与读回的机器
    /// 配置一致（`tag` 已校验），但重推一次只增加出错面，没有收益。
    pub(crate) fn from_stored(
        id_width: u8,
        align: u8,
        compressed_oops: bool,
        instance_header: u32,
        array_header: u32,
        class_shallow: u32,
    ) -> Option<Self> {
        let ok = matches!(id_width, 4 | 8)
            && align.is_power_of_two()
            && (4..=32).contains(&align)
            && instance_header > 0
            && array_header >= instance_header
            && class_shallow > 0;
        if !ok {
            return None;
        }
        Some(HeapLayout {
            id_width,
            align,
            compressed_oops,
            instance_header,
            array_header,
            class_shallow,
        })
    }

    /// 用 dump 里 `java.lang.Class` 的 instance_size 定类对象浅堆
    pub(crate) fn with_class_size(mut self, class_instance_size: Option<u64>) -> Self {
        self.class_shallow = match class_instance_size {
            Some(s) => self.instance_bytes(s) as u32,
            None => self.instance_header,
        };
        self
    }

    /// 实例浅堆：dump 给的 instance_size 向上对齐，且不小于对象头（0 说明类记录缺失）
    pub(crate) fn instance_bytes(&self, dump_instance_size: u64) -> u64 {
        let n = align_up(dump_instance_size, self.align as u64);
        n.max(self.instance_header as u64)
    }

    /// 数组浅堆：头 + n × 元素宽，整体对齐（`byte[3]` 是 16+3 → 24）
    pub(crate) fn array_bytes(&self, elems: u64, elem_width: u64) -> u64 {
        let body = elems.saturating_mul(elem_width);
        align_up(self.array_header as u64 + body, self.align as u64)
    }

    /// 口径指纹：**只覆盖可配置项**（对齐 / 压缩指针）。
    /// 索引头里存了整份布局，读回时用它判断「配置文件改过没有」—— id 宽度与
    /// 类对象浅堆来自 dump 本身，同一文件不会变，不该参与比较（否则索引永远失效）。
    pub(crate) fn tag(&self) -> u32 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in [self.align, self.compressed_oops as u8] {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        (h ^ (h >> 32)) as u32
    }

    /// 口径说明（前端展示 + 报告备注用，避免两侧各写一份文案）
    pub(crate) fn describe(&self) -> String {
        format!(
            "引用 {}B / {} / 对齐 {}B ⇒ 实例头 {}B、数组头 {}B、类对象 {}B",
            self.id_width,
            if self.id_width == 4 {
                "32 位"
            } else if self.compressed_oops {
                "压缩指针"
            } else {
                "普通指针"
            },
            self.align,
            self.instance_header,
            self.array_header,
            self.class_shallow
        )
    }

    /// 类名是否为 `java.lang.Class`（HPROF 内部名以 `/` 分隔）
    pub(crate) fn is_class_of_class(name: &str) -> bool {
        CLASS_OF_CLASS.iter().any(|c| *c == name)
    }
}

/// 当前配置的口径指纹（索引失效判定用；与具体 dump 的 id 宽度无关 ——
/// `tag()` 只看对齐与压缩指针，这两项在构造时已把配置默认值展开）
pub(crate) fn cfg_tag() -> u32 {
    let cfg = mat_cfg::layout_cfg();
    HeapLayout::with_params(8, cfg.alignment.unwrap_or(DEFAULT_ALIGN), cfg.compressed_oops).tag()
}

/// 向上对齐（`align` 已保证是 2 的幂）
fn align_up(n: u64, align: u64) -> u64 {
    if align <= 1 {
        return n;
    }
    (n + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三种指针模式的对象头（表见模块注释）
    #[test]
    fn headers_per_pointer_mode() {
        // 32 位：mark 4 + klass 4 = 8；数组 8+4=12 → 16
        let l32 = HeapLayout::with_params(4, 8, None);
        assert_eq!((l32.instance_header, l32.array_header), (8, 16));
        assert!(l32.compressed_oops);

        // 64 位压缩指针：8+4=12 → 16；数组 16 → 16
        let lc = HeapLayout::with_params(8, 8, Some(true));
        assert_eq!((lc.instance_header, lc.array_header), (16, 16));

        // 64 位普通指针：8+8=16；数组 20 → 24
        let lu = HeapLayout::with_params(8, 8, Some(false));
        assert_eq!((lu.instance_header, lu.array_header), (16, 24));

        // 显式指定压缩指针时，id 宽度 8 也按 4 字节 klass
        assert_ne!(lc.tag(), lu.tag());
    }

    /// 对齐粒度可覆盖（-XX:ObjectAlignmentInBytes=16），非法值回默认 8
    #[test]
    fn alignment_override_and_fallback() {
        let l16 = HeapLayout::with_params(8, 16, Some(true));
        assert_eq!((l16.instance_header, l16.array_header), (16, 16));
        assert_eq!(l16.array_bytes(1, 1), 32); // 16+1 → 32
        let bad = HeapLayout::with_params(8, 7, None);
        assert_eq!(bad.align, DEFAULT_ALIGN as u8);
    }

    /// 数组：头 + n×宽，整体对齐（byte[3] 占 24 而不是 19）
    #[test]
    fn array_sizes_are_aligned() {
        let l = HeapLayout::with_params(8, 8, Some(true));
        assert_eq!(l.array_bytes(0, 1), 16);
        assert_eq!(l.array_bytes(3, 1), 24);
        assert_eq!(l.array_bytes(4, 1), 24);
        assert_eq!(l.array_bytes(3, 8), 40); // long[3] = 16+24
        assert_eq!(l.array_bytes(2001, 2), 16 + 4002 + 6); // char[2001] → 对齐到 4024
        assert_eq!(l.array_bytes(2, 8), 32); // Object[2]（引用 8B）
    }

    /// 实例：dump 值向上对齐；0（类记录缺失）退回对象头
    #[test]
    fn instance_size_floor_is_header() {
        let l = HeapLayout::with_params(8, 8, Some(true));
        assert_eq!(l.instance_bytes(0), 16);
        assert_eq!(l.instance_bytes(20), 24);
        assert_eq!(l.instance_bytes(24), 24);
        assert_eq!(HeapLayout::with_params(4, 8, None).instance_bytes(8), 8);
    }

    /// 类对象浅堆：优先 java.lang.Class 的 instance_size，缺失退回对象头
    #[test]
    fn class_shallow_from_class_instance_size() {
        let l = HeapLayout::with_params(8, 8, Some(true)).with_class_size(Some(24));
        assert_eq!(l.class_shallow, 24);
        let l = HeapLayout::with_params(8, 8, Some(true)).with_class_size(None);
        assert_eq!(l.class_shallow, 16);
        assert!(HeapLayout::is_class_of_class("java/lang/Class"));
        assert!(HeapLayout::is_class_of_class("java.lang.Class"));
        assert!(!HeapLayout::is_class_of_class("java/lang/ClassLoader"));
    }

    /// 口径说明必须带上关键参数（前端直接展示，错了用户能一眼看出来）
    #[test]
    fn describe_mentions_headers() {
        let s = HeapLayout::with_params(8, 8, Some(false)).with_class_size(Some(24)).describe();
        assert!(s.contains("普通指针"), "{s}");
        assert!(s.contains("数组头 24B"), "{s}");
        assert!(s.contains("类对象 24B"), "{s}");
    }
}
