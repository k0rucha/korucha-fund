import secrets
from fastapi import Depends, HTTPException, status
from fastapi.security import HTTPBasic, HTTPBasicCredentials
from app.config import settings

security = HTTPBasic()


def verify_admin(credentials: HTTPBasicCredentials = Depends(security)) -> bool:
    valid_user = secrets.compare_digest(
        credentials.username.encode(), settings.admin_user.encode()
    )
    valid_pass = secrets.compare_digest(
        credentials.password.encode(), settings.admin_pass.encode()
    )
    if not (valid_user and valid_pass):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incorrect credentials",
            headers={"WWW-Authenticate": 'Basic realm="korucha-fund admin"'},
        )
    return True
