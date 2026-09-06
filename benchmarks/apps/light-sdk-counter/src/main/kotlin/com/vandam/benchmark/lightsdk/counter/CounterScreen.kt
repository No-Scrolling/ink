package com.vandam.benchmark.lightsdk.counter

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.thelightphone.sdk.ui.rememberLightTypography
import com.thelightphone.sdk.InitialScreen
import com.thelightphone.sdk.SealedLightActivity
import com.thelightphone.sdk.SimpleLightScreen
import com.thelightphone.sdk.ui.LightText
import com.thelightphone.sdk.ui.LightTextVariant
import com.thelightphone.sdk.ui.LightTheme
import com.thelightphone.sdk.ui.LightThemeTokens
import com.thelightphone.sdk.ui.LightTopBar
import com.thelightphone.sdk.ui.LightTopBarCenter
import com.thelightphone.sdk.ui.lightClickable

@InitialScreen
class CounterScreen(sealedActivity: SealedLightActivity) : SimpleLightScreen<Unit>(sealedActivity) {
    @Composable
    override fun Content() {
        var count by remember { mutableIntStateOf(0) }

        val density = LocalDensity.current
        val screenHeight = LocalConfiguration.current.screenHeightDp
        // Convert the shared 2.55 px design units to the SDK’s 600-unit vertical scale.
        val designScale = 2.55f / density.density * 600f / screenHeight
        val font = FontFamily(Font(com.vandam.benchmark.lightsdk.counter.R.font.public_sans_regular))
        val defaults = rememberLightTypography()
        val typography = defaults.copy(
            heading = defaults.heading.copy(
                fontFamily = font,
                fontSize = (40f * designScale).sp,
                lineHeight = (60f * designScale).sp,
            ),
            copy = defaults.copy.copy(
                fontFamily = font,
                fontSize = (30f * designScale).sp,
                lineHeight = (40f * designScale).sp,
            ),
            fine = defaults.fine.copy(
                fontFamily = font,
                fontSize = (20f * designScale).sp,
                letterSpacing = 0.sp,
            ),
        )

        LightTheme(typography = typography) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .background(LightThemeTokens.colors.background),
            ) {
                LightTopBar(center = LightTopBarCenter.Text("Counter"))
                Column(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .padding(
                            top = (6f * 2.55f / density.density).dp,
                            bottom = (20f * 2.55f / density.density).dp,
                        ),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy((16f * 2.55f / density.density).dp, Alignment.CenterVertically),
                ) {
                    LightText(text = "Count: $count", variant = LightTextVariant.Heading)
                    LightText(
                        text = "Increase",
                        variant = LightTextVariant.Copy,
                        modifier = Modifier.lightClickable { count += 1 },
                    )
                }
            }
        }
    }
}
