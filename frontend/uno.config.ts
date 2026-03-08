import { defineConfig, presetAttributify, presetIcons, presetWind3, transformerCompileClass } from 'unocss'

export default defineConfig({
    presets: [
        presetAttributify(),
        presetIcons(),
        presetWind3()
    ],
    transformers: [
        transformerCompileClass()
    ]
})