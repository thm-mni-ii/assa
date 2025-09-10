from typing import List, Optional, Union

import inflect


# Generate feedback for one submission against a set of reference solutions
def generate_prompt(
    sql_environment: str, 
    schema: str, 
    task: str, 
    solutions: List[str], 
    submission: str,
    feedback_language: str = "english",
    solution_results: Optional[List[Optional[Union[str, dict]]]] = None,
    submission_results: Optional[List[Optional[Union[str, dict]]]] = None
) -> str:
    p = inflect.engine()
    # Get the count for solutions
    n_solutions = len(solutions)
    # Dynamically determine the correct singular/plural wording for solutions
    solution_text = "reference solution" if n_solutions == 1 else "reference solutions"
    solutions_display_prefix = (
        "Reference Solution" if n_solutions == 1 else "Reference Solutions"
    )

    # Dynamically format solutions as a Markdown enumerated list if there are multiple
    if n_solutions > 1:
        solutions_list = "\n".join(
            [f"{i + 1}. {solution}" for i, solution in enumerate(solutions)]
        )
    else:
        solutions_list = f"{solutions[0]}" if solutions else "None provided"

    # Format results sections if provided
    solution_results_section = ""
    if solution_results is not None:
        solution_results_section = f"\nSolution Results:\n{solution_results}"
    
    submission_results_section = ""
    if submission_results is not None:
        submission_results_section = f"\nSubmission Results:\n{submission_results}"

    # Generate the prompt dynamically
    prompt_template = f"""
You are tasked with evaluating an SQL assignment. The task includes {p.number_to_words(str(n_solutions))} {solution_text} and one student submission. You are also provided with the SQL environment and database schema for context.

Your goal is to compare the student's submission step by step against the {solution_text}, identifying any syntax or semantic issues. Provide specific, actionable feedback that clearly explains the problems.

IMPORTANT: You have access to execution results for both the reference solutions and the student submission. Use these results to inform your analysis, but DO NOT reveal or mention the actual results, output data, or specific values in your feedback to the student. Focus on identifying correctness, logic issues, and providing educational guidance without exposing the expected or actual query results.

Please provide your feedback in {feedback_language}.

Context:
- SQL Environment: {sql_environment}
- Schema: {schema}
- Task: {task}

{solutions_display_prefix}:
{solutions_list}

Student Submission:
{submission}
{solution_results_section}
{submission_results_section}
"""
    return prompt_template.strip()
