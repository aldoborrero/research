# Cl@ve PIN App -- Design System
## Linear / Vercel Aesthetic for Flutter

---

## 1. COLOR SYSTEM

### Light Mode

| Role                  | Hex       | Usage                                      |
|-----------------------|-----------|---------------------------------------------|
| Background            | `#FAFAFA` | Main scaffold background                    |
| Surface               | `#FFFFFF` | Cards, bottom sheets, dialogs               |
| Surface Variant       | `#F5F5F5` | Secondary cards, input fields, code blocks  |
| Border                | `#E5E5E5` | Card borders, dividers, input outlines      |
| Border Subtle         | `#EBEBEB` | Very light separators between list items    |
| Text Primary          | `#171717` | Headlines, primary body text                |
| Text Secondary        | `#737373` | Captions, labels, helper text               |
| Text Tertiary         | `#A3A3A3` | Placeholder text, disabled states           |
| Accent                | `#5E6AD2` | Primary actions, links, active states       |
| Accent Hover          | `#4F5ABF` | Button hover / pressed states               |
| Success               | `#2DA44E` | Positive feedback, "PIN verified"           |
| Error                 | `#E5484D` | Error states, invalid PIN                   |
| Warning               | `#F5A623` | Caution states                              |

### Dark Mode

| Role                  | Hex       | Usage                                      |
|-----------------------|-----------|---------------------------------------------|
| Background            | `#0A0A0A` | Main scaffold background                    |
| Surface               | `#141414` | Cards, bottom sheets, dialogs               |
| Surface Variant       | `#1C1C1C` | Secondary cards, input fields               |
| Border                | `#2E2E2E` | Card borders, dividers                      |
| Border Subtle         | `#222222` | Very light separators                       |
| Text Primary          | `#EDEDED` | Headlines, primary body text                |
| Text Secondary        | `#A0A0A0` | Captions, labels, helper text               |
| Text Tertiary         | `#5C5C5C` | Placeholder text, disabled states           |
| Accent                | `#7B8ADE` | Primary actions, links, active states       |
| Accent Hover          | `#8E9BE5` | Button hover / pressed states               |
| Success               | `#3FB950` | Positive feedback                           |
| Error                 | `#F85149` | Error states                                |
| Warning               | `#D29922` | Caution states                              |

### Accent Color Options (pick ONE)

The Linear/Vercel aesthetic uses a single muted accent. Choose one:

| Name              | Hex Light   | Hex Dark    | Vibe                               |
|-------------------|-------------|-------------|-------------------------------------|
| **Indigo** (rec.) | `#5E6AD2`   | `#7B8ADE`   | Linear's signature -- calm authority |
| Violet            | `#7C3AED`   | `#A78BFA`   | Modern, creative                     |
| Blue              | `#2563EB`   | `#60A5FA`   | Trustworthy but NOT corporate blue   |
| Teal              | `#0D9488`   | `#2DD4BF`   | Fintech / fresh                      |
| Neutral (none)    | `#171717`   | `#EDEDED`   | Fully monochrome, buttons use text   |

**Recommendation for Cl@ve:** Use the **Indigo** (`#5E6AD2`) accent. It reads as
trustworthy without being generic "bank blue." It is the exact hue Linear uses and
pairs perfectly with the neutral gray scale.

---

## 2. TYPOGRAPHY

### Font Selection

**Primary:** `Inter` (Google Fonts -- `google_fonts` package)
- Why: Designed specifically for screens by Rasmus Andersson. Used by Linear, Vercel,
  Raycast, and most of the apps in this aesthetic. Large x-height, excellent
  readability at small sizes, 9 weights + variable.

**Alternative:** `DM Sans` (Google Fonts)
- Why: Slightly softer/friendlier than Inter. Good if the app needs to feel
  less "developer tool" and more "consumer fintech."

**Monospace (for PIN display):** `JetBrains Mono` or `Geist Mono` or `Space Mono` (Google Fonts)
- Why: PIN codes displayed in monospace feel intentional and technical. Space Mono
  is on Google Fonts and has character.

### Type Scale (8pt baseline grid)

All line heights are multiples of 4px. All sizes follow a 1.25 ratio (Major Third).

| Token          | Size  | Weight     | Line Height | Letter Spacing | Usage                        |
|----------------|-------|------------|-------------|----------------|-------------------------------|
| displayLarge   | 32px  | w700       | 40px        | -0.5px         | Hero text, onboarding titles  |
| displayMedium  | 24px  | w600       | 32px        | -0.3px         | Screen titles                 |
| titleLarge     | 20px  | w600       | 28px        | -0.2px         | Section headers               |
| titleMedium    | 16px  | w600       | 24px        | 0              | Card titles, dialog titles    |
| bodyLarge      | 16px  | w400       | 24px        | 0              | Primary body text             |
| bodyMedium     | 14px  | w400       | 20px        | 0              | Secondary body text           |
| bodySmall      | 12px  | w400       | 16px        | 0.2px          | Captions, helper text         |
| labelLarge     | 14px  | w500       | 20px        | 0.1px          | Button text                   |
| labelMedium    | 12px  | w500       | 16px        | 0.5px          | Overlines, badge text         |
| pinDigit       | 32px  | w600 mono  | 40px        | 8px            | PIN code digits               |

### Flutter Implementation

```dart
import 'package:google_fonts/google_fonts.dart';

TextTheme _buildTextTheme(Color textPrimary, Color textSecondary) {
  return TextTheme(
    displayLarge: GoogleFonts.inter(
      fontSize: 32,
      fontWeight: FontWeight.w700,
      height: 40 / 32,
      letterSpacing: -0.5,
      color: textPrimary,
    ),
    displayMedium: GoogleFonts.inter(
      fontSize: 24,
      fontWeight: FontWeight.w600,
      height: 32 / 24,
      letterSpacing: -0.3,
      color: textPrimary,
    ),
    titleLarge: GoogleFonts.inter(
      fontSize: 20,
      fontWeight: FontWeight.w600,
      height: 28 / 20,
      letterSpacing: -0.2,
      color: textPrimary,
    ),
    titleMedium: GoogleFonts.inter(
      fontSize: 16,
      fontWeight: FontWeight.w600,
      height: 24 / 16,
      color: textPrimary,
    ),
    bodyLarge: GoogleFonts.inter(
      fontSize: 16,
      fontWeight: FontWeight.w400,
      height: 24 / 16,
      color: textPrimary,
    ),
    bodyMedium: GoogleFonts.inter(
      fontSize: 14,
      fontWeight: FontWeight.w400,
      height: 20 / 14,
      color: textSecondary,
    ),
    bodySmall: GoogleFonts.inter(
      fontSize: 12,
      fontWeight: FontWeight.w400,
      height: 16 / 12,
      letterSpacing: 0.2,
      color: textSecondary,
    ),
    labelLarge: GoogleFonts.inter(
      fontSize: 14,
      fontWeight: FontWeight.w500,
      height: 20 / 14,
      letterSpacing: 0.1,
      color: textPrimary,
    ),
    labelMedium: GoogleFonts.inter(
      fontSize: 12,
      fontWeight: FontWeight.w500,
      height: 16 / 12,
      letterSpacing: 0.5,
      color: textSecondary,
    ),
  );
}
```

---

## 3. SPACING SYSTEM (8px Grid)

```dart
abstract class Spacing {
  static const double xxs = 2;
  static const double xs  = 4;
  static const double sm  = 8;
  static const double md  = 16;
  static const double lg  = 24;
  static const double xl  = 32;
  static const double xxl = 48;
  static const double xxxl = 64;

  // Page-level horizontal padding
  static const double pagePadding = 24;

  // Vertical spacing between major sections
  static const double sectionGap = 48;

  // Spacing between cards or list items
  static const double cardGap = 16;

  // Internal card padding
  static const double cardPadding = 20;  // slightly off-grid but feels right

  // Input field internal padding
  static const double inputPaddingH = 16;
  static const double inputPaddingV = 14;
}
```

### Key Spacing Rules

1. **Internal < External**: Padding inside a card must be less than the gap between cards.
2. **Generous whitespace**: When in doubt, add more space, not less. The Linear look
   comes from breathing room.
3. **Page padding**: 24px on mobile. On tablets/desktop, use 48px or more.
4. **Section titles to content**: 16px gap.
5. **Between form fields**: 16px vertical gap.
6. **Between a label and its field**: 8px.

---

## 4. COMPONENT PATTERNS

### 4.1 Cards (The Linear Way)

NO heavy shadows. Use a 1px border with slight background differentiation.

```dart
Container(
  padding: const EdgeInsets.all(20),
  decoration: BoxDecoration(
    color: Theme.of(context).colorScheme.surface,       // #FFFFFF light / #141414 dark
    borderRadius: BorderRadius.circular(12),
    border: Border.all(
      color: Theme.of(context).dividerColor,             // #E5E5E5 light / #2E2E2E dark
      width: 1,
    ),
  ),
  child: /* ... */,
)
```

**Key:** No `boxShadow`. The 1px border does all the work. If you must add a shadow,
make it extremely subtle:

```dart
boxShadow: [
  BoxShadow(
    color: Colors.black.withOpacity(0.04),  // barely visible
    blurRadius: 8,
    offset: const Offset(0, 2),
  ),
],
```

### 4.2 Buttons

**Primary Button (Accent)**
```dart
FilledButton(
  style: FilledButton.styleFrom(
    backgroundColor: const Color(0xFF5E6AD2),
    foregroundColor: Colors.white,
    padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
    shape: RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(8),
    ),
    elevation: 0,                                       // ZERO elevation
    textStyle: GoogleFonts.inter(
      fontSize: 14,
      fontWeight: FontWeight.w500,
    ),
  ),
  onPressed: () {},
  child: const Text('Verify PIN'),
)
```

**Secondary Button (Ghost / Outlined)**
```dart
OutlinedButton(
  style: OutlinedButton.styleFrom(
    foregroundColor: textPrimary,
    padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
    shape: RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(8),
    ),
    side: BorderSide(color: borderColor, width: 1),
    textStyle: GoogleFonts.inter(
      fontSize: 14,
      fontWeight: FontWeight.w500,
    ),
  ),
  onPressed: () {},
  child: const Text('Cancel'),
)
```

**Key button rules:**
- `elevation: 0` always. No Material shadow on buttons.
- Border radius: 8px (not 4px which is too sharp, not 16px+ which is too bubbly).
- Padding: generous horizontal (24px), comfortable vertical (14px).
- Font weight: w500 (medium), not bold.

### 4.3 Input Fields

```dart
TextField(
  decoration: InputDecoration(
    hintText: 'Enter your PIN',
    hintStyle: TextStyle(color: textTertiary),
    filled: true,
    fillColor: surfaceVariant,           // #F5F5F5 light / #1C1C1C dark
    contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
    border: OutlineInputBorder(
      borderRadius: BorderRadius.circular(8),
      borderSide: BorderSide(color: borderColor, width: 1),
    ),
    enabledBorder: OutlineInputBorder(
      borderRadius: BorderRadius.circular(8),
      borderSide: BorderSide(color: borderColor, width: 1),
    ),
    focusedBorder: OutlineInputBorder(
      borderRadius: BorderRadius.circular(8),
      borderSide: BorderSide(color: accent, width: 1.5),
    ),
    errorBorder: OutlineInputBorder(
      borderRadius: BorderRadius.circular(8),
      borderSide: const BorderSide(color: Color(0xFFE5484D), width: 1.5),
    ),
  ),
)
```

### 4.4 PIN Entry Display

The PIN entry is the hero of a Cl@ve app. Make it feel special but restrained.

```dart
/// Individual PIN dot / digit
Container(
  width: 48,
  height: 48,
  decoration: BoxDecoration(
    color: surfaceVariant,
    borderRadius: BorderRadius.circular(8),
    border: Border.all(
      color: isFocused ? accent : borderColor,
      width: isFocused ? 1.5 : 1,
    ),
  ),
  child: Center(
    child: isFilled
        ? Container(
            width: 12,
            height: 12,
            decoration: BoxDecoration(
              color: textPrimary,
              shape: BoxShape.circle,
            ),
          )
        : const SizedBox.shrink(),
  ),
)
```

**Spacing between PIN boxes:** 12px (Spacing.sm + Spacing.xs)

**Alternative: Underline style (more minimal)**
```dart
Container(
  width: 40,
  height: 48,
  alignment: Alignment.center,
  decoration: BoxDecoration(
    border: Border(
      bottom: BorderSide(
        color: isFocused ? accent : borderColor,
        width: isFocused ? 2 : 1,
      ),
    ),
  ),
  child: isFilled
      ? Text('*', style: TextStyle(fontSize: 24, color: textPrimary))
      : null,
)
```

### 4.5 App Bar (Minimal)

```dart
AppBar(
  backgroundColor: Colors.transparent,
  elevation: 0,
  scrolledUnderElevation: 0,         // prevents Material 3 scroll shadow
  centerTitle: true,
  title: Text(
    'Cl@ve PIN',
    style: GoogleFonts.inter(
      fontSize: 16,
      fontWeight: FontWeight.w600,
      color: textPrimary,
    ),
  ),
  leading: IconButton(
    icon: Icon(Icons.arrow_back, color: textPrimary, size: 20),
    onPressed: () => Navigator.pop(context),
  ),
)
```

### 4.6 Dividers

```dart
Divider(
  height: 1,
  thickness: 1,
  color: borderSubtle,   // #EBEBEB light / #222222 dark
)
```

### 4.7 Bottom Navigation / Tab Bar

```dart
NavigationBar(
  backgroundColor: surface,
  elevation: 0,
  indicatorColor: accent.withOpacity(0.1),
  surfaceTintColor: Colors.transparent,
  height: 64,
  labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
  // Add a top border instead of shadow:
  // Wrap in a Container with Border(top: ...)
)
```

---

## 5. BORDER RADIUS SYSTEM

```dart
abstract class Radii {
  static const double xs  = 4;    // Small chips, tags
  static const double sm  = 6;    // Tooltips, small popovers
  static const double md  = 8;    // Buttons, inputs, small cards
  static const double lg  = 12;   // Cards, dialogs
  static const double xl  = 16;   // Bottom sheets, large panels
  static const double full = 999; // Circular (avatars, dots)
}
```

**Rule:** Consistent radius. Pick 8px for most interactive elements, 12px for cards.
Do NOT use different radii on the same screen (it looks accidental).

---

## 6. ICONOGRAPHY

- **Style:** Outlined / line icons, NOT filled. Weight: 1.5px stroke.
- **Size:** 20px for inline, 24px for navigation, 16px for tight spaces.
- **Recommended packages:**
  - `lucide_icons` -- the icon set used by Vercel/shadcn. Clean, consistent 24x24 grid.
  - `hugeicons` -- large set with consistent line weight.
  - `phosphor_flutter` -- flexible, supports multiple weights.
- **Color:** Use `textSecondary` (#737373 light) for icons. Use `accent` only
  for active/selected states.

---

## 7. ANIMATION & MOTION

Linear/Vercel apps feel fast because animations are short and use ease-out curves.

```dart
abstract class Motion {
  static const Duration fast    = Duration(milliseconds: 150);
  static const Duration normal  = Duration(milliseconds: 200);
  static const Duration slow    = Duration(milliseconds: 300);

  static const Curve standard   = Curves.easeOut;
  static const Curve emphasize  = Curves.easeInOut;
  static const Curve enter      = Curves.easeOut;
  static const Curve exit       = Curves.easeIn;
}
```

**PIN entry animation:** When a digit is entered, use a subtle scale + fade:
```dart
AnimatedScale(
  scale: isFilled ? 1.0 : 0.0,
  duration: Motion.fast,
  curve: Motion.enter,
  child: /* dot */,
)
```

---

## 8. FULL THEME IMPLEMENTATION

```dart
import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

class AppTheme {
  // ---- Light Mode Colors ----
  static const _lightBackground    = Color(0xFFFAFAFA);
  static const _lightSurface       = Color(0xFFFFFFFF);
  static const _lightSurfaceVar    = Color(0xFFF5F5F5);
  static const _lightBorder        = Color(0xFFE5E5E5);
  static const _lightTextPrimary   = Color(0xFF171717);
  static const _lightTextSecondary = Color(0xFF737373);
  static const _lightTextTertiary  = Color(0xFFA3A3A3);

  // ---- Dark Mode Colors ----
  static const _darkBackground     = Color(0xFF0A0A0A);
  static const _darkSurface        = Color(0xFF141414);
  static const _darkSurfaceVar     = Color(0xFF1C1C1C);
  static const _darkBorder         = Color(0xFF2E2E2E);
  static const _darkTextPrimary    = Color(0xFFEDEDED);
  static const _darkTextSecondary  = Color(0xFFA0A0A0);
  static const _darkTextTertiary   = Color(0xFF5C5C5C);

  // ---- Accent ----
  static const accentLight         = Color(0xFF5E6AD2);
  static const accentDark          = Color(0xFF7B8ADE);

  // ---- Semantic ----
  static const success             = Color(0xFF2DA44E);
  static const error               = Color(0xFFE5484D);

  static ThemeData light() {
    return ThemeData(
      brightness: Brightness.light,
      scaffoldBackgroundColor: _lightBackground,
      colorScheme: const ColorScheme.light(
        surface: _lightSurface,
        surfaceContainerHighest: _lightSurfaceVar,
        primary: accentLight,
        onPrimary: Colors.white,
        onSurface: _lightTextPrimary,
        outline: _lightBorder,
        error: error,
      ),
      dividerColor: _lightBorder,
      textTheme: _buildTextTheme(_lightTextPrimary, _lightTextSecondary),
      appBarTheme: AppBarTheme(
        backgroundColor: Colors.transparent,
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: true,
        titleTextStyle: GoogleFonts.inter(
          fontSize: 16,
          fontWeight: FontWeight.w600,
          color: _lightTextPrimary,
        ),
        iconTheme: const IconThemeData(
          color: _lightTextPrimary,
          size: 20,
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: accentLight,
          foregroundColor: Colors.white,
          elevation: 0,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
          ),
          textStyle: GoogleFonts.inter(
            fontSize: 14,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: _lightTextPrimary,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
          ),
          side: const BorderSide(color: _lightBorder, width: 1),
          textStyle: GoogleFonts.inter(
            fontSize: 14,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: _lightSurfaceVar,
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: _lightBorder, width: 1),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: _lightBorder, width: 1),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: accentLight, width: 1.5),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: error, width: 1.5),
        ),
        hintStyle: GoogleFonts.inter(
          fontSize: 14,
          color: _lightTextTertiary,
        ),
      ),
      cardTheme: CardTheme(
        color: _lightSurface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: const BorderSide(color: _lightBorder, width: 1),
        ),
        margin: EdgeInsets.zero,
      ),
      dividerTheme: const DividerThemeData(
        color: _lightBorder,
        thickness: 1,
        space: 1,
      ),
      useMaterial3: true,
    );
  }

  static ThemeData dark() {
    return ThemeData(
      brightness: Brightness.dark,
      scaffoldBackgroundColor: _darkBackground,
      colorScheme: const ColorScheme.dark(
        surface: _darkSurface,
        surfaceContainerHighest: _darkSurfaceVar,
        primary: accentDark,
        onPrimary: Colors.white,
        onSurface: _darkTextPrimary,
        outline: _darkBorder,
        error: error,
      ),
      dividerColor: _darkBorder,
      textTheme: _buildTextTheme(_darkTextPrimary, _darkTextSecondary),
      appBarTheme: AppBarTheme(
        backgroundColor: Colors.transparent,
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: true,
        titleTextStyle: GoogleFonts.inter(
          fontSize: 16,
          fontWeight: FontWeight.w600,
          color: _darkTextPrimary,
        ),
        iconTheme: const IconThemeData(
          color: _darkTextPrimary,
          size: 20,
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: accentDark,
          foregroundColor: Colors.white,
          elevation: 0,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
          ),
          textStyle: GoogleFonts.inter(
            fontSize: 14,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: _darkTextPrimary,
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(8),
          ),
          side: const BorderSide(color: _darkBorder, width: 1),
          textStyle: GoogleFonts.inter(
            fontSize: 14,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: _darkSurfaceVar,
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: _darkBorder, width: 1),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: _darkBorder, width: 1),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: accentDark, width: 1.5),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: error, width: 1.5),
        ),
        hintStyle: GoogleFonts.inter(
          fontSize: 14,
          color: _darkTextTertiary,
        ),
      ),
      cardTheme: CardTheme(
        color: _darkSurface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: const BorderSide(color: _darkBorder, width: 1),
        ),
        margin: EdgeInsets.zero,
      ),
      dividerTheme: const DividerThemeData(
        color: _darkBorder,
        thickness: 1,
        space: 1,
      ),
      useMaterial3: true,
    );
  }

  static TextTheme _buildTextTheme(Color primary, Color secondary) {
    return TextTheme(
      displayLarge: GoogleFonts.inter(
        fontSize: 32, fontWeight: FontWeight.w700,
        height: 40 / 32, letterSpacing: -0.5, color: primary,
      ),
      displayMedium: GoogleFonts.inter(
        fontSize: 24, fontWeight: FontWeight.w600,
        height: 32 / 24, letterSpacing: -0.3, color: primary,
      ),
      titleLarge: GoogleFonts.inter(
        fontSize: 20, fontWeight: FontWeight.w600,
        height: 28 / 20, letterSpacing: -0.2, color: primary,
      ),
      titleMedium: GoogleFonts.inter(
        fontSize: 16, fontWeight: FontWeight.w600,
        height: 24 / 16, color: primary,
      ),
      bodyLarge: GoogleFonts.inter(
        fontSize: 16, fontWeight: FontWeight.w400,
        height: 24 / 16, color: primary,
      ),
      bodyMedium: GoogleFonts.inter(
        fontSize: 14, fontWeight: FontWeight.w400,
        height: 20 / 14, color: secondary,
      ),
      bodySmall: GoogleFonts.inter(
        fontSize: 12, fontWeight: FontWeight.w400,
        height: 16 / 12, letterSpacing: 0.2, color: secondary,
      ),
      labelLarge: GoogleFonts.inter(
        fontSize: 14, fontWeight: FontWeight.w500,
        height: 20 / 14, letterSpacing: 0.1, color: primary,
      ),
      labelMedium: GoogleFonts.inter(
        fontSize: 12, fontWeight: FontWeight.w500,
        height: 16 / 12, letterSpacing: 0.5, color: secondary,
      ),
    );
  }
}
```

---

## 9. SCREEN LAYOUT PATTERNS

### 9.1 Typical Screen Structure

```
+------------------------------------------+
|              (transparent AppBar)          |
|  <-  back                    Cl@ve PIN    |
+------------------------------------------+
|                                            |
|    [48px top padding]                      |
|                                            |
|    Display Title (32px, w700)              |
|    [8px gap]                               |
|    Subtitle text (14px, textSecondary)     |
|                                            |
|    [48px section gap]                      |
|                                            |
|    +------------------------------------+  |
|    |  Card with 1px border, 12px radius |  |
|    |  [20px internal padding]           |  |
|    |                                    |  |
|    |  Content here                      |  |
|    |                                    |  |
|    +------------------------------------+  |
|                                            |
|    [16px gap]                              |
|                                            |
|    +------------------------------------+  |
|    |  Another card                      |  |
|    +------------------------------------+  |
|                                            |
|                 (spacer)                   |
|                                            |
|    [====== Primary Button (full) ======]   |
|    [16px gap]                              |
|    [------ Secondary Button (full) ----]   |
|                                            |
|    [safe area bottom padding]              |
+------------------------------------------+
```

### 9.2 PIN Entry Screen Layout

```
+------------------------------------------+
|                                            |
|    [80px top]                              |
|                                            |
|    Enter your PIN  (24px, w600, centered) |
|    [8px]                                   |
|    Enter the PIN you use   (14px, gray)   |
|    to access Cl@ve          (centered)    |
|                                            |
|    [48px]                                  |
|                                            |
|         [ _ ] [ _ ] [ _ ] [ _ ]            |
|           48x48, 12px gap                  |
|                                            |
|    [32px]                                  |
|                                            |
|    [====== Verify PIN ================]    |
|                                            |
|    [24px]                                  |
|                                            |
|    Forgot your PIN?  (14px, accent)       |
|                        (centered, link)   |
|                                            |
+------------------------------------------+
```

---

## 10. DESIGN PRINCIPLES CHECKLIST

When building each screen, verify:

- [ ] **No shadows** on cards -- using 1px border + background color difference instead
- [ ] **No gradients** on buttons or backgrounds
- [ ] **elevation: 0** on all Material widgets (AppBar, Card, Button)
- [ ] **scrolledUnderElevation: 0** on AppBar to prevent scroll shadow
- [ ] **Inter font** everywhere, no mixing with other fonts (except mono for PIN)
- [ ] **8px grid** -- all spacing values are multiples of 4 or 8
- [ ] **Generous whitespace** -- at least 48px between major sections
- [ ] **Single accent color** -- only Indigo (#5E6AD2) for interactive elements
- [ ] **Gray text hierarchy** -- primary (#171717), secondary (#737373), tertiary (#A3A3A3)
- [ ] **Border radius consistency** -- 8px for buttons/inputs, 12px for cards
- [ ] **Icons** are outlined/line style, not filled
- [ ] **Animations** are under 200ms with easeOut curves
- [ ] **Dark mode** fully supported with proper dark equivalents

---

## 11. PUBSPEC DEPENDENCIES

```yaml
dependencies:
  google_fonts: ^6.2.1
  lucide_icons: ^0.257.0      # or phosphor_flutter
  # pin_code_fields or build custom
```

---

## 12. ANTI-PATTERNS TO AVOID

| Do NOT                                  | Do Instead                                    |
|----------------------------------------|-----------------------------------------------|
| Blue (#0066FF) as primary              | Muted indigo (#5E6AD2) as accent              |
| Drop shadows on cards                  | 1px border (#E5E5E5)                          |
| Rounded pill buttons (radius 999)      | Subtle rounded (radius 8)                     |
| Colorful status bar / app bar          | Transparent, blends with background           |
| Bold splashes of color everywhere      | Monochrome + ONE accent for actions only      |
| Material's default purple/teal scheme  | Custom neutral ColorScheme                    |
| Default Material elevation/shadows     | elevation: 0 everywhere                       |
| Tight spacing, cramped layouts         | Generous 24-48px breathing room               |
| Multiple font families                 | Inter only (+ mono for PIN digits)            |
| Filled/solid icons                     | Line/outlined icons (Lucide, Phosphor)        |
| Heavy onboarding illustrations         | Clean typography + subtle icon                |
| Gradient backgrounds                   | Flat solid neutral backgrounds                |
