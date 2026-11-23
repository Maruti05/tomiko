# Tomiko Assets

This directory is the single source of truth for packaged assets. Gradle adds it
to the app's `assets` source set; do not duplicate files under
`app/src/main/assets`.

Layout:

```text
assets/
├── characters/
│   └── <character-id>/
│       ├── character.glb        # rigged GLB, morph targets, animations
│       ├── thumbnail.webp       # 512x512 preview
│       └── metadata.json        # display name, scale, clip/morph mapping
├── environments/
│   └── studio.ktx2              # IBL environment
├── audio/
└── fonts/
```

Asset rules:

- GLB is the only character format.
- Textures must be KTX2/Basis Universal, power-of-two, and mipmapped.
- No copyrighted third-party character assets.
- Every character must pass the validation checks documented in
  `docs/CHARACTER_PIPELINE.md` before it is added to `characters/index.json`.
- Keep source `.blend` files and build scripts outside the APK.
