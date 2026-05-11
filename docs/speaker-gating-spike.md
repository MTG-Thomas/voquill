# Speaker Gating Spike

Tracking issue: https://github.com/MTG-Thomas/voquill/issues/3

## Goal

Evaluate whether Voquill can locally prefer the operator's voice and ignore nearby office speech without making dictation fragile.

## Candidate Approach

1. Capture a short local enrollment sample for the operator.
2. Use voice activity detection to split future recordings into speech segments.
3. Generate a local speaker embedding for each segment.
4. Compare segments against the enrolled operator embedding.
5. Keep likely-operator segments and warn on likely-background segments.

## Evaluation Questions

- Does the same user match reliably across laptop mic, Blue Yeti, and Yealink WH64 Pro?
- Does office noise or headset DSP cause false rejections?
- Can the model run locally without adding a large runtime or bloating the installer?
- Can the feature fail softly by warning instead of silently deleting speech?
- Is enrollment data stored locally in a clear, deletable app-data location?

## Candidate Libraries / Models To Research

- ONNX speaker verification models that can run through ONNX Runtime or Windows ML.
- pyannote-style speaker embeddings, if licensing and runtime size are acceptable.
- SpeechBrain ECAPA-TDNN variants exported to ONNX, if redistribution and performance are acceptable.

## First Proof Of Concept

Use an external scratch script, not production code:

1. Record three short samples from the operator on each microphone.
2. Record one or two nearby-speaker/background samples.
3. Compute embeddings locally.
4. Measure same-speaker versus different-speaker similarity margins.
5. Repeat with office headset DSP enabled.

## Recommendation Gate

Do not add production speaker gating unless the proof of concept shows a comfortable margin between operator and non-operator speech across all intended microphones. If margins are weak, keep Office Mode plus VAD/correction learning as the safer path.
