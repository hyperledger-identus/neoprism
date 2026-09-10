set working-directory := '../..'

# Compare a published NeoPRISM image with a candidate on Preprod
run CANDIDATE_IMAGE *ARGS:
    cd tools && python -m indexer_parity.cli --candidate-image "{{ CANDIDATE_IMAGE }}" {{ ARGS }}

# Run offline tests for manifest normalization and comparison
test:
    cd tools && python -m unittest discover -s indexer_parity/tests
