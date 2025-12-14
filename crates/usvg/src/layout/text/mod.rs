        // If chunks is empty then we have just a whitespace.
        if chunks.is_empty() {
            return None;
        }

        let base_transform = text.abs_transform();
        let mut spans = Vec::new();

        let mut iter = chunks.iter();
        while let Some(chunk) = iter.next() {
            // Skip empty text chunks
            if chunk.text.is_empty() {
                continue;
            }

            let mut chunk_spans = match &chunk.text_flow {
