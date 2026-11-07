import android.graphics.Paint;
import android.graphics.Typeface;
import android.graphics.text.LineBreaker;
import android.graphics.text.MeasuredText;
import android.graphics.text.PositionedGlyphs;
import android.graphics.text.TextRunShaper;
import java.util.Arrays;
import java.util.HashMap;

// Standalone API probe, run with app_process on the emulator. This deliberately
// does not claim to include Ink's JNI transfer or atlas upload costs.
public final class AndroidTextProbe {
    private static final String[] TEXT = {
        "Count: 0", "office ffi", "AVATAR", "1234567890",
        "مرحبا بالعالم", "🙂", "one\ttwo", "e\u0301"
    };

    private static PositionedGlyphs shape(String text, Paint paint) {
        return TextRunShaper.shapeTextRun(text, 0, text.length(), 0, text.length(),
            0f, 0f, text.startsWith("مرحبا"), paint);
    }

    public static void main(String[] args) throws Exception {
        // app_process does not run the font-map initialisation performed when
        // Android binds an application. This is probe setup, not an app API.
        java.lang.reflect.Method initialise = Typeface.class.getDeclaredMethod("loadPreinstalledSystemFontMap");
        initialise.setAccessible(true);
        initialise.invoke(null);
        Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG | Paint.SUBPIXEL_TEXT_FLAG | Paint.LINEAR_TEXT_FLAG);
        paint.setHinting(Paint.HINTING_OFF);
        paint.setTypeface(Typeface.createFromFile(args[0]));
        // ab_glyph's 46px scale is ascent-to-descent, Android's is em size.
        paint.setTextSize(46f * 2000f / 2350f);
        for (String text : TEXT) {
            PositionedGlyphs glyphs = shape(text, paint);
            char[] chars = text.toCharArray();
            MeasuredText measured = new MeasuredText.Builder(chars)
                .appendStyleRun(paint, chars.length, text.startsWith("مرحبا"))
                .build();
            LineBreaker breaker = new LineBreaker.Builder()
                .setBreakStrategy(LineBreaker.BREAK_STRATEGY_SIMPLE).build();
            LineBreaker.ParagraphConstraints constraints = new LineBreaker.ParagraphConstraints();
            constraints.setWidth(180f);
            LineBreaker.Result lines = breaker.computeLineBreaks(measured, constraints, 0);
            System.out.println("metric\t" + text + "\t" + glyphs.getAdvance()
                + "\t" + measured.getWidth(0, chars.length) + "\t" + glyphs.glyphCount()
                + "\t" + lines.getLineCount());
        }
        paint.setTypeface(Typeface.DEFAULT);
        for (String text : TEXT) System.out.println("system\t" + text + "\t" + shape(text, paint).getAdvance());
        paint.setTypeface(Typeface.createFromFile(args[0]));
        for (int i = 0; i < 2000; i++) shape(TEXT[i % TEXT.length], paint);
        for (boolean cached : new boolean[] {false, true}) {
            long[] samples = new long[100];
            HashMap<String, PositionedGlyphs> previous = new HashMap<>();
            long calls = 0;
            float checksum = 0f;
            for (int frame = 0; frame < samples.length; frame++) {
                HashMap<String, PositionedGlyphs> current = new HashMap<>();
                long start = System.nanoTime();
                for (int i = 0; i < 1000; i++) {
                    String text = TEXT[i % TEXT.length];
                    PositionedGlyphs glyphs = cached ? current.get(text) : null;
                    if (glyphs == null && cached) glyphs = previous.get(text);
                    if (glyphs == null) { glyphs = shape(text, paint); calls++; }
                    if (cached) current.put(text, glyphs);
                    checksum += glyphs.getAdvance();
                }
                samples[frame] = System.nanoTime() - start;
                previous = current;
            }
            Arrays.sort(samples);
            System.out.println("timing\t" + cached + "\t" + samples[49] + "\t"
                + samples[94] + "\t" + calls + "\t" + checksum);
        }
    }
}
