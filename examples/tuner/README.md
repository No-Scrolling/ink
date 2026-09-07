# Tuner

A simple chromatic tuner for the Light Phone III, inspired by the implemented features of [Light Strings](https://github.com/garado/light-strings).

From this directory, run:

```sh
ink dev
```

Tuner checks microphone permission when it opens and shows the system permission prompt if needed. Allow access to begin tuning. Play one sustained note at a time. The tuner shows the nearest note and octave, frequency, and cents above or below pitch. The tick indicator sits left of centre when flat (tune up), and right when sharp (tune down). Within ±5 cents, it settles at the taller centre mark. Once permission is granted, listening starts when you open the tuner. The last detected reading stays visible during silence; placeholders appear only before the first reading of the session. Changing the reference pitch recalculates that reading.

Open **Settings** to show flats instead of sharps, hide or show cents and frequency independently, or open **Reference pitch** to choose an A4 reference between 400 and 480 Hz (440 Hz by default). Cents and frequency are hidden by default. Press the keypad’s **Done** action to save the reference. Your choices are kept between launches; leaving the reference screen without pressing Done discards that edit.

Microphone capture stops when the tuner screen is hidden or the app leaves the foreground. Listening resumes automatically when you return to the app. Sound is analysed locally; no recording is saved. Pitch detection uses Ink’s native audio module, with an approximate range of 50–1,200 Hz. Quiet notes, background noise and strong harmonics can affect detection.
