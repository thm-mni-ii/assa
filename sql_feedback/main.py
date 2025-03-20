from typing import List

from fastapi import FastAPI, HTTPException
from openai import OpenAI

from models import AnalyzeRequest, AnalyzeResponse, SQLAnalyzerOutput
from settings import Settings
from utils import generate_prompt

# Initialize FastAPI and external clients
app = FastAPI()
settings = Settings()

client = OpenAI(base_url=settings.base_url, api_key=settings.openai_api_key)
system_prompt = "You are a helpful assistant who is an excellent educator for SQL."


@app.post(
    "/api/v1/analyse",
    response_model=List[AnalyzeResponse],
    summary="Analyze SQL Submission",
    description="Analyze a user's SQL submissions against provided solutions for correctness.",
    response_description="Result of the analysis, including correctness and feedback.",
)
async def analyze_sql(request: AnalyzeRequest) -> List[AnalyzeResponse]:
    """
    Endpoint to analyze a user's SQL submission(s) against a set of solutions.
    - **sql_environment**: Target SQL environment or dialect.
    - **task**: Task description or requirements.
    - **solutions**: Array of correct SQL solutions.
    - **submissions**: Array of user's submitted SQL code.
    - **schema**: Database schema including tables, keys, and other relevant details.
    """
    sql_environment = request.sql_environment
    schema = request.db_schema
    task = request.task
    solutions = request.solutions
    submissions = request.submissions

    # Validate submissions
    if not isinstance(submissions, list) or not submissions:
        raise HTTPException(
            status_code=400,
            detail="Submissions must be a non-empty list of SQL queries.",
        )

    results = []  # To gather responses for each submission

    for submission in submissions:
        # Create a prompt for feedback generation for the current submission
        prompt = generate_prompt(sql_environment, schema, task, solutions, submission)

        # Prepare chat messages
        user_messages = [{"role": "user", "content": prompt}]
        messages = [{"role": "system", "content": system_prompt}] + user_messages
        messages.append({"role": "assistant", "content": ""})

        # API call
        try:
            completion = client.beta.chat.completions.parse(
                model="thm/gemma-3-27b-Q8",
                messages=messages,
                response_format=SQLAnalyzerOutput,
                temperature=0,
                top_p=0.95,
            )
            response: SQLAnalyzerOutput | None = completion.choices[0].message.parsed

            if response:
                correct = response.student_feedback.overall_correctness
                results.append(
                    AnalyzeResponse(
                        correct=correct,
                        feedback=response.student_feedback.overall_feedback
                        if not correct
                        else "",
                    )
                )
            else:
                raise Exception("Error processing submission: LLM response empty")
        except Exception as e:
            results.append(
                AnalyzeResponse(
                    correct=False, feedback=f"Error processing submission: {str(e)}"
                )
            )

    return results
