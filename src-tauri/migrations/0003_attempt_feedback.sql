-- Add tutor feedback column to attempt table (nullable — populated after LLM call)
ALTER TABLE attempt ADD COLUMN tutor_feedback TEXT;
