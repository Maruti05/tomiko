# Tomiko — UI/UX Design System (Material 3)

## 1. Design Principles

| Principle | Application |
|---|---|
| **Kid-friendly** | Large touch targets (≥ 56 dp), bright colors, rounded shapes |
| **Minimal text** | Icon-driven navigation; read-aloud labels where text exists |
| **Confidence building** | Positive feedback animations, celebratory sounds no "wrong" states |
| **Safety** | Parental gate before settings; no external links |
| **Accessibility** | Minimum 12:1 contrast ratio, large fonts (≥ 20 sp body) |

---

## 2. Material 3 Theming

### 2.1 Color Palette

```kotlin
// presentation/theme/Color.kt
// Vibrant, child-friendly palette with dynamic color support

// Light Theme
val md_theme_light_primary = Color(0xFF6C5CE7)        // Purple
val md_theme_light_onPrimary = Color(0xFFFFFFFF)
val md_theme_light_primaryContainer = Color(0xFFE8E0FF)
val md_theme_light_onPrimaryContainer = Color(0xFF22005D)
val md_theme_light_secondary = Color(0xFFFF6B6B)       // Coral
val md_theme_light_onSecondary = Color(0xFFFFFFFF)
val md_theme_light_tertiary = Color(0xFF00B894)        // Teal
val md_theme_light_background = Color(0xFFFFF8E7)      // Warm cream
val md_theme_light_surface = Color(0xFFFFF8E7)
val md_theme_light_onBackground = Color(0xFF1C1B1F)
val md_theme_light_error = Color(0xFFE74C3C)

// Dark Theme
val md_theme_dark_primary = Color(0xFFCBB8FF)
val md_theme_dark_onPrimary = Color(0xFF3B0091)
val md_theme_dark_primaryContainer = Color(0xFF5300C7)
val md_theme_dark_secondary = Color(0xFFFF9E9E)
val md_theme_dark_background = Color(0xFF1C1B1F)
val md_theme_dark_surface = Color(0xFF1C1B1F)
val md_theme_dark_onBackground = Color(0xFFE6E1E5)
```

### 2.2 Typography

```kotlin
// presentation/theme/Type.kt
val KidTypography = Typography(
    displayLarge = TextStyle(
        fontWeight = FontWeight.Bold,
        fontSize = 48.sp,
        lineHeight = 56.sp,
        letterSpacing = (-0.25).sp
    ),
    headlineLarge = TextStyle(
        fontWeight = FontWeight.Bold,
        fontSize = 36.sp,
        lineHeight = 44.sp
    ),
    titleLarge = TextStyle(
        fontWeight = FontWeight.SemiBold,
        fontSize = 28.sp,
        lineHeight = 36.sp
    ),
    titleMedium = TextStyle(
        fontWeight = FontWeight.SemiBold,
        fontSize = 22.sp,
        lineHeight = 28.sp
    ),
    bodyLarge = TextStyle(
        fontWeight = FontWeight.Normal,
        fontSize = 20.sp,
        lineHeight = 28.sp
    ),
    bodyMedium = TextStyle(
        fontWeight = FontWeight.Normal,
        fontSize = 18.sp,
        lineHeight = 24.sp
    ),
    labelLarge = TextStyle(
        fontWeight = FontWeight.Medium,
        fontSize = 18.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.5.sp
    ),
)
```

### 2.3 Shapes

```kotlin
// presentation/theme/Shape.kt
val KidShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(20.dp),
    large = RoundedCornerShape(28.dp),
    extraLarge = RoundedCornerShape(36.dp),
)
```

---

## 3. Edge-to-Edge & System Bars

```kotlin
// MainActivity.kt
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.light(
                android.graphics.Color.TRANSPARENT,
                android.graphics.Color.TRANSPARENT
            ),
            navigationBarStyle = SystemBarStyle.light(
                android.graphics.Color.TRANSPARENT,
                android.graphics.Color.TRANSPARENT
            )
        )
        setContent {
            TomikoTheme {
                TomikoApp()
            }
        }
    }
}
```

All composables apply `Modifier.windowInsetsPadding(WindowInsets.systemBars)` or `WindowInsets.displayCutout()` as appropriate. The 3D viewport intentionally draws **behind** the navigation bar for immersion.

---

## 4. Adaptive Layout Strategy

### 4.1 Window Size Classes

```kotlin
@Composable
fun TalkScreen(characterId: String, navController: NavController) {
    val windowSizeClass = currentWindowAdaptiveInfo().windowSizeClass

    when {
        windowSizeClass.windowWidthSizeClass == WindowWidthSizeClass.COMPACT -> {
            // Phone portrait: Character top, controls bottom
            VerticalTalkLayout(characterId)
        }
        windowSizeClass.windowWidthSizeClass == WindowWidthSizeClass.MEDIUM -> {
            // Phone landscape / small tablet
            HorizontalTalkLayout(characterId)
        }
        windowSizeClass.windowWidthSizeClass == WindowWidthSizeClass.EXPANDED -> {
            // Tablet / foldable unfolded
            TabletTalkLayout(characterId)
        }
    }
}
```

### 4.2 Layout Variants

| Size Class | Layout | 3D Viewport | Controls |
|---|---|---|---|
| **Compact** (≤ 600 dp) | Vertical stack | 60 % height | Full-width bottom bar |
| **Medium** (600–840 dp) | Horizontal split | 50 % left | Right panel |
| **Expanded** (≥ 840 dp) | Horizontal with sidebar | 60 % left | Right panel + side controls |

### 4.3 Foldable Posture Detection

```kotlin
@Composable
fun TalkScreenWithFoldable(characterId: String) {
    val windowInfo = rememberWindowLayoutInfo()
    val posture = windowInfo.windowLayoutInfo.displayFeatures
        .filterIsInstance<FoldingFeature>()
        .firstOrNull()

    if (posture != null && posture.isSeparating) {
        // Dual-screen: character on one screen, controls on the other
        FoldableDualScreenLayout(
            characterId,
            posture.bounds,
            posture.orientation
        )
    } else {
        // Normal layout based on size class
    }
}
```

---

## 5. Screen Compositions

### 5.1 Splash Screen

```
┌──────────────────────────────┐
│  ┌────────────────────────┐  │
│  │                        │  │
│  │    Logo (animated      │  │
│  │     character icon)    │  │
│  │                        │  │
│  └────────────────────────┘  │
│                              │
│       "Tomiko" (text)        │
│     ──────────────           │
│     Loading... 80%           │
└──────────────────────────────┘
```

- Auto-dismisses when `SceneManager` confirms character load
- Animated circular progress indicator
- Preloads first character in background

### 5.2 Home Screen (Character Select)

```
┌──────────────────────────────┐
│ [⚙️]                    [ℹ️] │
│                              │
│    ┌──┐  ┌──┐  ┌──┐         │
│    │T │  │R │  │B │         │
│    │om│  │ab│  │un│         │
│    │iko│  │bit│  │ny│         │
│    └──┘  └──┘  └──┘         │
│   "Tomiko" "Rabbit" "Bunny" │
│                              │
│   ─────────────────────      │
│     "Pick your friend!"      │
└──────────────────────────────┘
```

- 3D character thumbnails rendered as SceneView snapshots
- Horizontal scrollable carousel
- Tap → navigate to TalkScreen

### 5.3 Talk Screen (Main Interaction)

**Phone Portrait:**
```
┌──────────────────────────────┐
│ ┌──────────────────────────┐ │
│ │                          │ │
│ │    3D CHARACTER          │ │
│ │    (SceneView)           │ │
│ │                          │ │
│ └──────────────────────────┘ │
│                              │
│   ┌──────────────────────┐  │
│   │  Waveform ▁▂▃▄▅▆▇██  │  │
│   └──────────────────────┘  │
│                              │
│         ┌────────┐          │
│         │   🎤   │          │
│         │  TALK  │          │
│         └────────┘          │
│                              │
│  [🔄 Replay]  [🔊 Repeat]   │
└──────────────────────────────┘
```

**Tablet Landscape:**
```
┌──────────────────────────────────────────────┐
│ ┌──────────────────────────┐  ┌────────────┐ │
│ │                          │  │  Contols:  │ │
│ │    3D CHARACTER          │  │  🎤 Talk   │ │
│ │    (SceneView)           │  │  🔊 Replay │ │
│ │                          │  │  🔄 Repeat │ │
│ └──────────────────────────┘  └────────────┘ │
│  ▁▂▃▄▅▆▇███ Waveform                           │
└──────────────────────────────────────────────┘
```

### 5.4 Settings Screen

```
┌──────────────────────────────┐
│        ← Settings            │
│                              │
│  ┌──────────────────────┐   │
│  │ Parental Gate:       │   │
│  │ "What is 3 + 5?"     │   │
│  │ [__answer__] [✓]     │   │
│  └──────────────────────┘   │
│                              │
│  Character: Tomiko    [🔄]  │
│  Volume:    [=========o]    │
│  Clear Data [Reset]         │
│  Version 1.0.0              │
└──────────────────────────────┘
```

---

## 6. Component Specs

| Component | Min Size | Touch Target | Animation |
|---|---|---|---|
| RecordButton | 80×80 dp | 56×56 dp | Pulse scale 1→1.1 during recording |
| CharacterCard | 120×160 dp | same | Elevation lift on hover |
| Back button | 48×48 dp | 56×56 dp | Fade in/out |
| Volume slider | full width | 48 dp height | Material 3 slider |

---

## 7. Dark Mode

- Full M3 dark theme supported
- Automatic based on system setting (no in-app toggle)
- 3D viewport background adjusts to dark surface color
- All icons use `@drawable/ic_*_light` / `@drawable/ic_*_dark` variants
