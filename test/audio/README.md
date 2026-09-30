# Audio processor checks

The `audio` suite calls the production `LevelProcessor` and `PitchProcessor` through `AudioProcessor`. Expected levels come from silence and a half-duty full-scale signal; expected notes come from generated tones with known frequencies. The tests do not reproduce peak/RMS traversal or pitch detection.

Cases protect configuration/reset, signed PCM limits, frequency-to-note/reference behaviour, sample-rate changes and rejection of silence/DC as a musical note. Clean tones must produce the expected note within five cents (the documented tuning indicator's centred range), with frequency error below 0.3%. These are fixture acceptance criteria, not precision guarantees for microphone recordings. The initial one-Hz absolute tolerance was discarded: it imposed a tighter musical tolerance at higher pitches without a supported accuracy contract.

These host checks exclude microphone capture, playback, codecs, effects, Android audio routing and acoustic latency.
