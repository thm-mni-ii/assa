from typing import List, Optional, Union, Any

from pydantic import BaseModel, Field


Results = List[Optional[Union[str, dict]]]


class AnalyzeRequest(BaseModel):
    sql_environment: str = Field(
        ...,
        title="SQL Environment",
        description="Target SQL environment/dialect.",
    )
    db_schema: str = Field(
        ...,
        title="Database Schema",
        description="Database schema including tables, keys, and other relevant details.",
    )
    task: str = Field(
        ...,
        title="Task Description",
        description="Task description or requirements.",
    )
    solutions: List[str] = Field(
        ...,
        title="Solutions",
        description="Array of correct SQL solutions.",
    )
    submissions: List[str] = Field(
        ...,
        title="Submission",
        description="User's submitted SQL code.",
    )
    solution_results: Optional[Results] = Field(
        default=None,
        title="Solution Results",
        description="Execution results for the solution queries.",
    )
    submission_results: Optional[Results] = Field(
        default=None,
        title="Submission Results", 
        description="Execution results for the submitted queries.",
    )
    task_id: Optional[str] = Field(
        default=None,
        title="Task ID",
        description="Unique identifier for the task.",
    )
    user_id: Optional[str] = Field(
        default=None,
        title="User ID",
        description="Unique identifier for the user.",
    )
    feedback_language: Optional[str] = Field(
        default=None,
        title="Feedback Language",
        description="The language the generated feedback is in."
    )


class AnalyzeResponse(BaseModel):
    correct: bool
    feedback: str


class Step(BaseModel):
    explanation: str = Field(
        description="A detailed explanation of the issue or analysis performed for this step."
    )
    output: str = Field(
        description="The resulting output or an example output related to this step, if applicable."
    )


class SubmissionFeedback(BaseModel):
    student_submission: str = Field(
        description="The SQL query submitted by the student for evaluation."
    )
    syntax_analysis: List[Step] = Field(
        description="A list of identified syntax issues or analyses performed on the SQL code."
    )
    semantics_analysis: List[Step] = Field(
        description="A list of identified semantic issues or logical errors in the SQL query."
    )
    overall_feedback: str = Field(
        description="A high-level summary of issues, analysis, and actionable suggestions for improvement."
    )
    overall_correctness: bool = Field(
        description="Indicates whether the student's submission is correct ('true') or incorrect ('false') based on the evaluation."
    )


class SQLAnalyzerOutput(BaseModel):
    task: str = Field(
        description="A description of the SQL task or objective for which the submission is being evaluated."
    )
    reference_solutions: List[str] = Field(
        description="A list of reference SQL solutions considered correct for the given task."
    )
    student_feedback: SubmissionFeedback = Field(
        description="A detailed feedback report, including syntax and semantic analyses, for the student's submission. Must be in the specified language."
    )
